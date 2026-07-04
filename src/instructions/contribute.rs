use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::Transfer, state::Mint};

use crate::{
    constants::{MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER, SECONDS_TO_DAYS},
    error::FundraiserError,
    state::{Contributor, Fundraiser},
};

pub fn process_contribute_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        contributor,
        mint_to_raise,
        fundraiser_account,
        contributor_account,
        contributor_ata,
        vault,
        _token_program,
        _system_program,
        _remaining_accounts @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());

    let decimals = Mint::from_account_view(mint_to_raise)?.decimals();

    let fundraiser_key = *fundraiser_account.address();

    let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;

    if fundraiser_state.mint_to_raise != *mint_to_raise.address().as_array() {
        return Err(ProgramError::InvalidAccountData);
    }
    let expected_fundraiser = derive_address(
        &[b"fundraiser".as_ref(), fundraiser_state.maker.as_ref()],
        Some(fundraiser_state.bump),
        &crate::ID.to_bytes(),
    );
    if expected_fundraiser != *fundraiser_key.as_array() {
        return Err(ProgramError::InvalidSeeds);
    }

    let max_contribution =
        fundraiser_state.amount_to_raise() * MAX_CONTRIBUTION_PERCENTAGE / PERCENTAGE_SCALER;

    let (contributor_pda, contributor_bump) = Address::find_program_address(
        &[b"contributor", fundraiser_key.as_ref(), contributor.address().as_ref()],
        &crate::ID,
    );
    if &contributor_pda != contributor_account.address() {
        return Err(ProgramError::InvalidSeeds);
    }
    if !contributor_account.owned_by(&crate::ID) {
        let bump_bytes = [contributor_bump];
        let signer_seeds = [
            Seed::from(b"contributor"),
            Seed::from(fundraiser_key.as_ref()),
            Seed::from(contributor.address().as_ref()),
            Seed::from(&bump_bytes),
        ];
        CreateAccount {
            from: contributor,
            to: contributor_account,
            lamports: Rent::get()?.try_minimum_balance(Contributor::LEN)?,
            space: Contributor::LEN as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[Signer::from(&signer_seeds)])?;
    }

    let contributor_state = Contributor::from_account_info(contributor_account)?;

    if !(amount > 1u64.pow(decimals as u32)) {
        return Err(FundraiserError::ContributionTooSmall.into());
    }
    if !(amount <= max_contribution) {
        return Err(FundraiserError::ContributionTooBig.into());
    }
    let now = Clock::get()?.unix_timestamp;
    let elapsed_days = ((now - fundraiser_state.time_started()) / SECONDS_TO_DAYS) as u8;
    // reversed in anchor-fundraiser (likely a bug); this direction makes more sense
    if elapsed_days >= fundraiser_state.duration {
        return Err(FundraiserError::FundraiserEnded.into());
    }
    if !(contributor_state.amount() <= max_contribution
        && contributor_state.amount() + amount <= max_contribution)
    {
        return Err(FundraiserError::MaximumContributionsReached.into());
    }

    Transfer::new(contributor_ata, vault, contributor, amount).invoke()?;

    fundraiser_state.set_current_amount(
        fundraiser_state
            .current_amount()
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    );
    contributor_state.set_amount(
        contributor_state
            .amount()
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    );

    Ok(())
}

use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};
use pinocchio_pubkey::derive_address;
use pinocchio_token::{instructions::Transfer, state::Account as TokenAccount};

use crate::{
    constants::SECONDS_TO_DAYS,
    error::FundraiserError,
    state::{Contributor, Fundraiser},
};

pub fn process_refund_instruction(accounts: &mut [AccountView], _data: &[u8]) -> ProgramResult {
    let [
        contributor,
        maker,
        mint_to_raise,
        fundraiser_account,
        contributor_account,
        contributor_ata,
        vault,
        _token_program,
        _remaining_accounts @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let fundraiser_key = *fundraiser_account.address();

    let (fmint, amount_to_raise, duration, time_started, bump, current_amount) = {
        let f = Fundraiser::from_account_info(fundraiser_account)?;
        (
            f.mint_to_raise,
            f.amount_to_raise(),
            f.duration,
            f.time_started(),
            f.bump,
            f.current_amount(),
        )
    };

    if fmint != *mint_to_raise.address().as_array() {
        return Err(ProgramError::InvalidAccountData);
    }
    let expected_fundraiser = derive_address(
        &[b"fundraiser".as_ref(), maker.address().as_ref()],
        Some(bump),
        &crate::ID.to_bytes(),
    );
    if expected_fundraiser != *fundraiser_key.as_array() {
        return Err(ProgramError::InvalidSeeds);
    }

    let now = Clock::get()?.unix_timestamp;
    let elapsed_days = ((now - time_started) / SECONDS_TO_DAYS) as u8;
    // reversed in anchor-fundraiser (likely a bug); this direction makes more sense
    if elapsed_days < duration {
        return Err(FundraiserError::FundraiserNotEnded.into());
    }

    let vault_amount = {
        let v = TokenAccount::from_account_view(vault)?;
        if v.mint() != mint_to_raise.address() || v.owner() != fundraiser_account.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        v.amount()
    };
    if !(vault_amount < amount_to_raise) {
        return Err(FundraiserError::TargetMet.into());
    }

    let refund_amount = {
        let c = Contributor::from_account_info(contributor_account)?;
        c.amount()
    };

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&bump_bytes),
    ];
    Transfer::new(vault, contributor_ata, fundraiser_account, refund_amount)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    {
        let f = Fundraiser::from_account_info(fundraiser_account)?;
        f.set_current_amount(
            current_amount
                .checked_sub(refund_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?,
        );
    }

    contributor.set_lamports(contributor.lamports() + contributor_account.lamports());
    contributor_account.close()?;

    Ok(())
}

use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_associated_token_account::instructions::Create as CreateAta;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::Mint;

use crate::{constants::MIN_AMOUNT_TO_RAISE, error::FundraiserError, state::Fundraiser};

pub fn process_initialize_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser_account,
        vault,
        system_program,
        token_program,
        _associated_token_program,
        _remaining_accounts @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let duration = data[8];

    let decimals = Mint::from_account_view(mint_to_raise)?.decimals();
    if !(amount > MIN_AMOUNT_TO_RAISE.pow(decimals as u32)) {
        return Err(FundraiserError::InvalidAmount.into());
    }

    let (fundraiser_pda, bump) =
        Address::find_program_address(&[b"fundraiser", maker.address().as_ref()], &crate::ID);
    if &fundraiser_pda != fundraiser_account.address() {
        return Err(ProgramError::InvalidSeeds);
    }
    if fundraiser_account.owned_by(&crate::ID) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&bump_bytes),
    ];
    CreateAccount {
        from: maker,
        to: fundraiser_account,
        lamports: Rent::get()?.try_minimum_balance(Fundraiser::LEN)?,
        space: Fundraiser::LEN as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[Signer::from(&signer_seeds)])?;

    CreateAta {
        funding_account: maker,
        account: vault,
        wallet: fundraiser_account,
        mint: mint_to_raise,
        system_program,
        token_program,
    }
    .invoke()?;

    let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
    fundraiser_state.set_maker(maker.address());
    fundraiser_state.set_mint_to_raise(mint_to_raise.address());
    fundraiser_state.set_amount_to_raise(amount);
    fundraiser_state.set_current_amount(0);
    fundraiser_state.set_time_started();
    fundraiser_state.set_duration(duration);
    fundraiser_state.set_bump(bump);

    Ok(())
}

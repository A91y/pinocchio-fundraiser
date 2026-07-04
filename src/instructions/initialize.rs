use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::{Account as TokenAccount, Mint};

use crate::{constants::MIN_AMOUNT_TO_RAISE, error::FundraiserError, state::Fundraiser};

pub fn process_initialize_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser_account,
        vault,
        _system_program,
        _remaining_accounts @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < 10 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let duration = data[8];
    let bump = data[9];

    let decimals = Mint::from_account_view(mint_to_raise)?.decimals();
    if !(amount > MIN_AMOUNT_TO_RAISE.pow(decimals as u32)) {
        return Err(FundraiserError::InvalidAmount.into());
    }

    let expected_fundraiser = derive_address(
        &[b"fundraiser".as_ref(), maker.address().as_ref()],
        Some(bump),
        &crate::ID.to_bytes(),
    );
    if expected_fundraiser != *fundraiser_account.address().as_array() {
        return Err(ProgramError::InvalidSeeds);
    }
    if fundraiser_account.owned_by(&crate::ID) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // The vault is created by the client; validate it belongs to this fundraiser + mint.
    {
        let v = TokenAccount::from_account_view(vault)?;
        if v.owner() != fundraiser_account.address() || v.mint() != mint_to_raise.address() {
            return Err(ProgramError::InvalidAccountData);
        }
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

    let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
    fundraiser_state.set_maker(maker.address());
    fundraiser_state.set_mint_to_raise(mint_to_raise.address());
    fundraiser_state.set_amount_to_raise(amount);
    fundraiser_state.set_current_amount(0);
    fundraiser_state.set_time_started();
    fundraiser_state.set_duration(duration);
    fundraiser_state.set_bump(bump);
    fundraiser_state.set_vault(vault.address());

    Ok(())
}

use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_pubkey::derive_address;
use pinocchio_token::{instructions::Transfer, state::Account as TokenAccount};

use crate::{error::FundraiserError, state::Fundraiser};

pub fn process_check_contributions_instruction(
    accounts: &mut [AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser_account,
        vault,
        maker_ata,
        token_program,
        system_program,
        _associated_token_program,
        _remaining_accounts @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let (amount_to_raise, bump) = {
        let f = Fundraiser::from_account_info(fundraiser_account)?;
        (f.amount_to_raise(), f.bump)
    };

    let expected_fundraiser = derive_address(
        &[b"fundraiser".as_ref(), maker.address().as_ref()],
        Some(bump),
        &crate::ID.to_bytes(),
    );
    if expected_fundraiser != *fundraiser_account.address().as_array() {
        return Err(ProgramError::InvalidSeeds);
    }

    CreateIdempotent {
        funding_account: maker,
        account: maker_ata,
        wallet: maker,
        mint: mint_to_raise,
        system_program,
        token_program,
    }
    .invoke()?;

    let vault_amount = {
        let v = TokenAccount::from_account_view(vault)?;
        if v.mint() != mint_to_raise.address() || v.owner() != fundraiser_account.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        v.amount()
    };

    if !(vault_amount >= amount_to_raise) {
        return Err(FundraiserError::TargetNotMet.into());
    }

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&bump_bytes),
    ];
    Transfer::new(vault, maker_ata, fundraiser_account, vault_amount)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    maker.set_lamports(maker.lamports() + fundraiser_account.lamports());
    fundraiser_account.close()?;

    Ok(())
}

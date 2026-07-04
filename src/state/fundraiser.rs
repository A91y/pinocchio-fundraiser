use pinocchio::{AccountView, Address, error::ProgramError};
use pinocchio::sysvars::{Sysvar, clock::Clock};

// byte arrays keep alignment at 1 so the raw account buffer can be cast in place (zero-copy)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fundraiser {
    pub maker: [u8; 32],
    pub mint_to_raise: [u8; 32],
    pub amount_to_raise: [u8; 8],
    pub current_amount: [u8; 8],
    pub time_started: [u8; 8],
    pub duration: u8,
    pub bump: u8,
}

impl Fundraiser {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 1 + 1;

    pub fn from_account_info(account_info: &mut AccountView) -> Result<&mut Self, ProgramError> {
        let data = unsafe { account_info.borrow_unchecked_mut() };
        if data.len() != Fundraiser::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn set_maker(&mut self, maker: &Address) {
        self.maker.copy_from_slice(maker.as_ref());
    }

    pub fn set_mint_to_raise(&mut self, mint_to_raise: &Address) {
        self.mint_to_raise.copy_from_slice(mint_to_raise.as_ref());
    }

    pub fn set_amount_to_raise(&mut self, amount_to_raise: u64) {
        self.amount_to_raise.copy_from_slice(&amount_to_raise.to_le_bytes());
    }
    
    pub fn set_current_amount(&mut self, current_amount: u64) {
        self.current_amount.copy_from_slice(&current_amount.to_le_bytes());
    }

    pub fn set_time_started(&mut self) {
        let timestamp = Clock::get().unwrap().unix_timestamp;
        self.time_started.copy_from_slice(&timestamp.to_le_bytes());
    }
    
    pub fn set_duration(&mut self, duration: u8) {
        self.duration = duration;
    }

    pub fn set_bump(&mut self, bump: u8) {
        self.bump = bump;
    }

    pub fn amount_to_raise(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_raise)
    }

    pub fn current_amount(&self) -> u64 {
        u64::from_le_bytes(self.current_amount)
    }

    pub fn time_started(&self) -> i64 {
        i64::from_le_bytes(self.time_started)
    }
}

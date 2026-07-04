use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FundraiserError {
    TargetNotMet = 0,
    TargetMet = 1,
    ContributionTooBig = 2,
    ContributionTooSmall = 3,
    MaximumContributionsReached = 4,
    FundraiserNotEnded = 5,
    FundraiserEnded = 6,
    InvalidAmount = 7,
}

impl From<FundraiserError> for ProgramError {
    fn from(e: FundraiserError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowError {
    InvalidInstruction = 0,
    InvalidState = 1,
    InvalidMaker = 2,
    InvalidTaker = 3,
    InvalidMint = 4,
    InvalidVault = 5,
    InvalidTokenAccount = 6,
    InvalidAmount = 7,
    ArithmeticOverflow = 8,
}

impl From<EscrowError> for ProgramError {
    fn from(value: EscrowError) -> Self {
        ProgramError::Custom(value as u32)
    }
}

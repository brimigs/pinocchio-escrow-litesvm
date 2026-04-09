use pinocchio::error::ProgramError;

use crate::error::EscrowError;

pub const MAKE_DISCRIMINATOR: u8 = 0;
pub const TAKE_DISCRIMINATOR: u8 = 1;
pub const CANCEL_DISCRIMINATOR: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MakeArgs {
    pub seed: u64,
    pub offered_amount: u64,
    pub expected_amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowInstruction {
    Make(MakeArgs),
    Take,
    Cancel,
}

impl EscrowInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        let (discriminator, rest) = data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        match *discriminator {
            MAKE_DISCRIMINATOR => {
                if rest.len() != 24 {
                    return Err(ProgramError::InvalidInstructionData);
                }

                let seed = u64::from_le_bytes(rest[0..8].try_into().unwrap());
                let offered_amount = u64::from_le_bytes(rest[8..16].try_into().unwrap());
                let expected_amount = u64::from_le_bytes(rest[16..24].try_into().unwrap());

                if offered_amount == 0 || expected_amount == 0 {
                    return Err(EscrowError::InvalidAmount.into());
                }

                Ok(Self::Make(MakeArgs {
                    seed,
                    offered_amount,
                    expected_amount,
                }))
            }
            TAKE_DISCRIMINATOR if rest.is_empty() => Ok(Self::Take),
            CANCEL_DISCRIMINATOR if rest.is_empty() => Ok(Self::Cancel),
            _ => Err(EscrowError::InvalidInstruction.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_make_instruction() {
        let mut data = [0u8; 25];
        data[0] = MAKE_DISCRIMINATOR;
        data[1..9].copy_from_slice(&7u64.to_le_bytes());
        data[9..17].copy_from_slice(&50u64.to_le_bytes());
        data[17..25].copy_from_slice(&25u64.to_le_bytes());

        assert_eq!(
            EscrowInstruction::unpack(&data).unwrap(),
            EscrowInstruction::Make(MakeArgs {
                seed: 7,
                offered_amount: 50,
                expected_amount: 25,
            })
        );
    }

    #[test]
    fn reject_zero_amount_make_instruction() {
        let mut data = [0u8; 25];
        data[0] = MAKE_DISCRIMINATOR;
        data[1..9].copy_from_slice(&7u64.to_le_bytes());
        data[9..17].copy_from_slice(&0u64.to_le_bytes());
        data[17..25].copy_from_slice(&25u64.to_le_bytes());

        assert_eq!(
            EscrowInstruction::unpack(&data),
            Err(EscrowError::InvalidAmount.into())
        );
    }
}

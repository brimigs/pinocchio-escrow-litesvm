use pinocchio::{error::ProgramError, Address};

use crate::error::EscrowError;

pub const ESCROW_SEED_PREFIX: &[u8] = b"escrow";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowState {
    pub bump: u8,
    pub seed: u64,
    pub maker: Address,
    pub mint_a: Address,
    pub mint_b: Address,
    pub offered_amount: u64,
    pub expected_amount: u64,
}

impl EscrowState {
    pub const DISCRIMINATOR: u8 = 1;
    pub const LEN: usize = 1 + 1 + 8 + 32 + 32 + 32 + 8 + 8;

    pub fn new(
        bump: u8,
        seed: u64,
        maker: Address,
        mint_a: Address,
        mint_b: Address,
        offered_amount: u64,
        expected_amount: u64,
    ) -> Self {
        Self {
            bump,
            seed,
            maker,
            mint_a,
            mint_b,
            offered_amount,
            expected_amount,
        }
    }

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() < Self::LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }

        dst[0] = Self::DISCRIMINATOR;
        dst[1] = self.bump;
        dst[2..10].copy_from_slice(&self.seed.to_le_bytes());
        dst[10..42].copy_from_slice(self.maker.as_ref());
        dst[42..74].copy_from_slice(self.mint_a.as_ref());
        dst[74..106].copy_from_slice(self.mint_b.as_ref());
        dst[106..114].copy_from_slice(&self.offered_amount.to_le_bytes());
        dst[114..122].copy_from_slice(&self.expected_amount.to_le_bytes());

        Ok(())
    }

    pub fn unpack(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != Self::LEN {
            return Err(EscrowError::InvalidState.into());
        }

        if src[0] != Self::DISCRIMINATOR {
            return Err(EscrowError::InvalidState.into());
        }

        Ok(Self {
            bump: src[1],
            seed: u64::from_le_bytes(src[2..10].try_into().unwrap()),
            maker: Address::new_from_array(src[10..42].try_into().unwrap()),
            mint_a: Address::new_from_array(src[42..74].try_into().unwrap()),
            mint_b: Address::new_from_array(src[74..106].try_into().unwrap()),
            offered_amount: u64::from_le_bytes(src[106..114].try_into().unwrap()),
            expected_amount: u64::from_le_bytes(src[114..122].try_into().unwrap()),
        })
    }

    pub fn escrow_address(&self, program_id: &Address) -> (Address, u8) {
        let seed_bytes = self.seed.to_le_bytes();
        Address::find_program_address(
            &[ESCROW_SEED_PREFIX, self.maker.as_ref(), &seed_bytes],
            program_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_state() {
        let state = EscrowState::new(
            254,
            9,
            Address::new_from_array([1u8; 32]),
            Address::new_from_array([2u8; 32]),
            Address::new_from_array([3u8; 32]),
            50,
            25,
        );
        let mut data = [0u8; EscrowState::LEN];

        state.pack(&mut data).unwrap();

        assert_eq!(EscrowState::unpack(&data).unwrap(), state);
    }
}

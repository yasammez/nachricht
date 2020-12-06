use crate::error::DecodeError;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum Fixed {
    Unit = 0,
    True = 1,
    False = 2,
    F32 = 3,
    F64 = 4,
}

impl Fixed {

    #[inline]
    pub fn from_bits(data: u64) -> Result<Self, DecodeError> {
        match data {
            x if x == Fixed::Unit as u64 => Ok(Fixed::Unit),
            x if x == Fixed::True as u64 => Ok(Fixed::True),
            x if x == Fixed::False as u64 => Ok(Fixed::False),
            x if x == Fixed::F32 as u64 => Ok(Fixed::F32),
            x if x == Fixed::F64 as u64 => Ok(Fixed::F64),
            i => Err(DecodeError::FixedValue(i)),
        }
    }

    #[inline]
    pub fn to_bits(&self) -> u64 {
        *self as u64
    }

}

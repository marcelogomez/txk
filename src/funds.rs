use std::ops::Neg;

use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum FundsOpError {
    #[error("Overflow")]
    Overflow,
}

/// Wrapper type for overflow safe operations to represent funds
///
/// This type represents a trade-off between API ergonomics and safety.
/// by making the innter type private and not implementing traits like
/// `Deref`, `Add`, `Sub`, etc. we make dealing with funds safer (against overflows)
/// but harder to use. Normally overflows would either cause a `panic` or, arguably worse,
/// happen silently. This type makes it so that arithmetic operations are fallible so we handle
/// overflows explicitly.
///
/// Arguably overflows are rare enough that this it not worth it,
/// but this at least serves as an illustration of how to use the type system
/// to implement these tradeoffs.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy, Deserialize)]
pub struct Funds(Decimal);

impl Neg for Funds {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(self.0.neg())
    }
}

impl From<Funds> for Decimal {
    fn from(funds: Funds) -> Self {
        funds.0
    }
}

impl Funds {
    pub fn new<T: Into<Decimal>>(n: T) -> Self {
        Self(n.into())
    }

    pub fn add<T: Into<Decimal>>(&self, n: T) -> Result<Self, FundsOpError> {
        Ok(Self(
            self.0.checked_add(n.into()).ok_or(FundsOpError::Overflow)?,
        ))
    }

    pub fn sub<T: Into<Decimal>>(&self, n: T) -> Result<Self, FundsOpError> {
        Ok(Self(
            self.0.checked_sub(n.into()).ok_or(FundsOpError::Overflow)?,
        ))
    }

    pub fn is_negative(&self) -> bool {
        self.0.is_sign_negative()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pos_overflow() {
        assert_eq!(
            Funds::new(Decimal::MAX).add(42),
            Err(FundsOpError::Overflow),
        );
    }

    #[test]
    fn test_neg_overflow() {
        assert_eq!(
            Funds::new(Decimal::MIN).sub(42),
            Err(FundsOpError::Overflow),
        );
    }

    #[test]
    fn test_neg() {
        assert_eq!(-Funds::new(Decimal::MIN), Funds::new(-Decimal::MIN));
        assert_eq!(-Funds::new(Decimal::MAX), Funds::new(-Decimal::MAX));
    }
}

use crate::funds::{Funds, FundsOpError};
use rust_decimal::Decimal;

/// Type to represent the internal funds balance of an account
///
/// This restricts the API for changing the balances to make it safer through immutability
/// By making this an immutable `Copy` type we make it safe for changes to either balance which can fail
/// independently without needing to implement rollback logic.
///
/// Note that although the only possible failure in the current implementation is either balance overflowing
/// the same API can be extended to guard against errors such as maintaining a minimum balance or held funds
/// not being negative
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Balance {
    available: Funds,
    held: Funds,
}

impl Balance {
    pub fn new() -> Self {
        Self {
            available: Funds::new(0),
            held: Funds::new(0),
        }
    }

    pub fn available(&self) -> Funds {
        self.available
    }

    pub fn held(&self) -> Funds {
        self.held
    }

    pub fn apply(self, diff: BalanceDiff) -> Result<Self, FundsOpError> {
        Ok(Self {
            available: match diff.available {
                Some(da) => self.available.add(da)?,
                None => self.available,
            },
            held: match diff.held {
                Some(dh) => self.held.add(dh)?,
                None => self.held,
            },
        })
    }
}

impl Default for Balance {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a change to an account's `Balance`
#[derive(Debug, PartialEq, Eq)]
pub struct BalanceDiff {
    available: Option<Funds>,
    held: Option<Funds>,
}

impl BalanceDiff {
    pub fn new() -> Self {
        Self {
            available: None,
            held: None,
        }
    }

    pub fn with_available<T: Into<Decimal>>(self, da: T) -> Self {
        Self {
            available: Some(Funds::new(da)),
            held: self.held,
        }
    }

    pub fn with_held<T: Into<Decimal>>(self, dh: T) -> Self {
        Self {
            available: self.available,
            held: Some(Funds::new(dh)),
        }
    }
}

impl Default for BalanceDiff {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::funds::Funds;
    use crate::funds::FundsOpError;

    #[test]
    fn test_balance_apply() {
        assert_eq!(
            Balance::new().apply(BalanceDiff::new().with_available(100).with_held(-100)),
            Ok(Balance {
                available: Funds::new(100),
                held: Funds::new(-100),
            }),
        );
    }

    #[test]
    fn test_balance_apply_overflow() {
        assert_eq!(
            Balance::new()
                .apply(
                    BalanceDiff::new()
                        .with_available(Decimal::MAX)
                        .with_held(Decimal::MAX)
                )
                .expect("To succeed")
                .apply(BalanceDiff::new().with_available(1)),
            Err(FundsOpError::Overflow),
        );
    }
}

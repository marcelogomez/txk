use crate::funds::Funds;
use serde::Deserialize;

pub type ClientID = u16;
pub type TransactionID = u32;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// Type for representing transactions from the csv input files
///
/// Ideally this would be an enum to account for the fact that
/// amount is only presennt for deposits and withdrawals.
/// However, the csv crate does not deal very well with tagged enum
/// deserialization (see https://github.com/BurntSushi/rust-csv/issues/278).
///
/// Instead we opt to make amount an `Option`
///
/// This has some implications for serialisation:
/// because all records need to have the same amount of columns we need a trailing comma for
/// records that do not have an amount
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: ClientID,
    #[serde(rename = "tx")]
    pub transaction: TransactionID,
    pub amount: Option<Funds>,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::funds::Funds;
    use csv::Reader;

    fn deserialize_transaction_from_str(t: &str) -> Transaction {
        Reader::from_reader(format!("type,client,tx,amount\n{}", t).as_bytes())
            .deserialize::<Transaction>()
            .next()
            .expect("One element")
            .expect("Serialization to succeed")
    }

    #[test]
    fn test_deserialize_deposit() {
        assert_eq!(
            deserialize_transaction_from_str("deposit,1,1,1.0"),
            Transaction {
                tx_type: TransactionType::Deposit,
                client: 1,
                transaction: 1,
                amount: Some(Funds::new(1)),
            },
        );
    }

    #[test]
    fn test_deserialize_dispute() {
        assert_eq!(
            // Note that we need a trailing comma. This is because amount is an Option
            deserialize_transaction_from_str("dispute,1,1,"),
            Transaction {
                tx_type: TransactionType::Dispute,
                client: 1,
                transaction: 1,
                amount: None,
            },
        );
    }
}

pub type ClientID = u16;
pub type TransactionID = u32;

#[derive(Debug, PartialEq)]
pub enum Transaction {
    Deposit {
        client: ClientID,
        transaction: TransactionID,
        amount: f32,
    },
    Withdrawal {
        client: ClientID,
        transaction: TransactionID,
        amount: f32,
    },
    Dispute {
        client: ClientID,
        transaction: TransactionID,
    },
    Resolve {
        client: ClientID,
        transaction: TransactionID,
    },
    Chargeback {
        client: ClientID,
        transaction: TransactionID,
    },
}
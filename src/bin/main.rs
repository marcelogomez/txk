use std::convert::TryFrom;
use std::path::Path;

use clap::Parser;
use csv::Reader;
use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;
use txk::account::Account;
use txk::transaction::ClientID;
use txk::transaction::Transaction;
use txk::transaction_engine::TransactionEngine;

// TODO: Move this to balance or serialisation
const MAX_DEC_DIGITS: u32 = 4;

#[derive(Serialize)]
struct OutRecord {
    client: ClientID,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl OutRecord {
    fn new(account: &Account) -> Self {
        let available: Decimal = account.balance().available().into();
        let held: Decimal = account.balance().held().into();
        let total = available + held;
        Self {
            client: account.client_id(),
            available: available.round_dp(MAX_DEC_DIGITS),
            held: held.round_dp(MAX_DEC_DIGITS),
            total: total.round_dp(MAX_DEC_DIGITS),
            locked: account.is_frozen(),
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    input_file: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut engine = TransactionEngine::new();
    for transaction in Reader::from_path(Path::new(&args.input_file))?.deserialize::<Transaction>()
    {
        match transaction {
            Ok(t) => {
                if let Err(e) = engine.process(t) {
                    eprintln!("Failed to process transaction: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse transaction: {}", e);
            }
        }
    }

    let mut out = Writer::from_writer(std::io::stdout());
    for account in engine.accounts().values() {
        if let Err(e) = out.serialize(OutRecord::new(account)) {
            eprintln!(
                "Failed to serialize record for account {}: {}",
                account.client_id(),
                e
            );
        }
    }

    Ok(())
}

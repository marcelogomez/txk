use csv::Reader;
use txk::transaction::Transaction;
use txk::transaction_engine::TransactionEngine;

fn main() -> anyhow::Result<()> {
    let mut engine = TransactionEngine::new();
    for transaction in Reader::from_reader(std::io::stdin()).deserialize::<Transaction>() {
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

    println!("{:?}", engine);
    Ok(())
}

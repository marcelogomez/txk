use csv::Reader;
use txk::transaction::Transaction;
fn main() -> anyhow::Result<()> {
    for transaction in Reader::from_reader(std::io::stdin()).deserialize::<Transaction>() {
        println!("{:?}", transaction?);
    }
    Ok(())
}

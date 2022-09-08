use clap::Parser;
use csv::Reader;
use csv::ReaderBuilder;
use csv::Trim;
use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use txk::account::Account;
use txk::transaction::ClientID;
use txk::transaction::Transaction;
use txk::transaction_engine::TransactionEngine;

// TODO: Move this to balance or serialisation
const MAX_DEC_DIGITS: u32 = 4;

const NUM_THREADS: usize = 8;

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
    #[clap(short, long, default_value_t = NUM_THREADS)]
    num_threads: usize,
    input_file: String,
}

fn receiver_thread(out: Sender<anyhow::Result<OutRecord>>, input: Receiver<Transaction>) {
    let mut engine = TransactionEngine::new();

    for transaction in input {
        // Forward errors to be logged
        if let Err(e) = engine.process(transaction) {
            let _ = out.send(Err(anyhow::anyhow!(e)));
        }
    }

    for account in engine.accounts().values() {
        let _ = out.send(Ok(OutRecord::new(account)));
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Set up output channel
    let (out_sender, out_receiver) = channel::<anyhow::Result<OutRecord>>();

    // Set up processing threads
    let num_threads = args.num_threads;
    let mut input_senders = vec![];
    let mut receiver_threads = vec![];
    for (sender, receiver) in std::iter::repeat_with(|| channel::<Transaction>()).take(num_threads)
    {
        input_senders.push(sender);
        let out = out_sender.clone();
        receiver_threads.push(std::thread::spawn(move || {
            receiver_thread(out, receiver);
        }));
    }

    let mut reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_path(Path::new(&args.input_file))?;

    // Route input from file into the right thread based on the client id
    for transaction in reader.deserialize::<Transaction>()
    {
        match transaction {
            Ok(t) => {
                let thread_num = (t.client as usize) % num_threads;
                let _ = input_senders[thread_num].send(t);
            }
            // Forward error to be logged
            Err(e) => {
                let _ = out_sender.send(Err(anyhow::anyhow!(e)));
            }
        }
    }

    // Need to drop senders for receiver threads to finish
    drop(input_senders);
    drop(out_sender);

    let mut out = Writer::from_writer(std::io::stdout());
    for record in out_receiver {
        match record {
            Ok(r) => {
                if let Err(e) = out.serialize(&r) {
                    eprintln!("Failed to seralize record for account {}: {}", r.client, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to process transaction: {}", e);
            }
        }
    }

    Ok(())
}

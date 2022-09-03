use log::error;
use std::{env, io};

mod bursar;
use crate::bursar::{Bursar, Transaction};

fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        error!("Exactly one argument is supported");
        std::process::exit(1);
    }
    let file_path = std::path::Path::new(&args[1]);
    if !file_path.exists() {
        error!("File path does not exist");
        std::process::exit(1);
    }

    let mut reader = csv::Reader::from_path(file_path).expect("Could not read csv file");

    let tx_iter = reader.deserialize::<Transaction>().filter_map(|item| {
        if item.is_err() {
            error!("could not parse transaction, will be skipped: {:?}", item)
        }
        item.ok()
    });

    let mut bursar = Bursar::new();
    bursar.consume(tx_iter);
    bursar.write_results(io::stdout());
}

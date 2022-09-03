use csv::WriterBuilder;
use log::error;
use rust_decimal::prelude::*;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::io;

struct Client {
    client_id: u16,
    available: Decimal,
    held: Decimal,
    locked: bool,
}

impl Client {
    fn new(client_id: u16) -> Self {
        Client {
            client_id,
            available: Decimal::default(),
            held: Decimal::default(),
            locked: false,
        }
    }

    fn total(&self) -> Decimal {
        self.available + self.held
    }

    fn deposit(&mut self, amount: &Decimal) {
        self.available += amount
    }

    fn withdraw(&mut self, amount: &Decimal) {
        self.available -= amount
    }

    fn dispute(&mut self, amount: &Decimal) {
        self.available -= amount;
        self.held += amount;
    }

    fn resolve(&mut self, amount: &Decimal) {
        self.available += amount;
        self.held -= amount;
    }

    fn chargeback(&mut self, amount: &Decimal) {
        self.held -= amount;
        self.locked = true;
    }
}

impl Serialize for Client {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Client", 4)?;
        state.serialize_field("client", &self.client_id)?;
        state.serialize_field("available", &self.available.round_dp(4).to_string())?;
        state.serialize_field("held", &self.held.round_dp(4).to_string())?;
        state.serialize_field("total", &self.total().round_dp(4).to_string())?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    #[serde(alias = "type")]
    tx_type: Op,
    #[serde(alias = "client")]
    client_id: u16,
    #[serde(alias = "tx")]
    tx_id: u32,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    amount: Option<Decimal>,
}

impl Transaction {
    pub fn new(tx_type: Op, client_id: u16, tx_id: u32, amount: Option<Decimal>) -> Self {
        Transaction {
            tx_type,
            client_id,
            tx_id,
            amount,
        }
    }
}

pub struct Bursar {
    transactions: HashMap<u32, Option<Decimal>>,
    clients: HashMap<u16, Client>,
    disputed: HashSet<u32>,
}

impl Bursar {
    pub fn new() -> Self {
        Bursar {
            transactions: HashMap::new(),
            clients: HashMap::new(),
            disputed: HashSet::new(),
        }
    }

    pub fn consume(&mut self, transactions: impl Iterator<Item = Transaction>) {
        transactions.for_each(|tx| self.process_transaction(tx));
    }

    pub fn process_transaction(&mut self, tx: Transaction) {
        let client = self
            .clients
            .entry(tx.client_id)
            .or_insert_with(|| Client::new(tx.client_id));

        let amount = match tx.tx_type {
            Op::Deposit | Op::Withdrawal => {
                // keep amount of transaction that might be referenced to
                self.transactions.entry(tx.tx_id).or_insert(tx.amount);
                &tx.amount
            }
            Op::Dispute => {
                self.disputed.insert(tx.tx_id);
                self.transactions.get(&tx.tx_id).unwrap_or(&None)
            }
            Op::Resolve | Op::Chargeback => {
                if self.disputed.contains(&tx.tx_id) {
                    // retrieve amount associated to referenced transaction
                    self.transactions.get(&tx.tx_id).unwrap_or(&None)
                } else {
                    // the resolve or chargeback is referencing a undisputed transaction
                    &None
                }
            }
        };
        if let Some(amount) = amount {
            match tx.tx_type {
                Op::Deposit => client.deposit(amount),
                Op::Withdrawal => client.withdraw(amount),
                Op::Dispute => client.dispute(amount),
                Op::Resolve => client.resolve(amount),
                Op::Chargeback => client.chargeback(amount),
            }
        } else {
            error!("transactions '{:?}' is not valid", tx.tx_id);
        }
    }

    pub fn write_results<T: io::Write>(&mut self, target: T) {
        let mut writer = WriterBuilder::new().from_writer(target);
        self.clients.values().for_each(|client| {
            writer
                .serialize(client)
                .expect("Unable to serialize client");
        });
        writer.flush().expect("Unable to write to target");
    }
}

#[cfg(test)]
use rust_decimal_macros::dec;

#[test]
fn sanity() {
    let mut bursar = Bursar::new();
    let client_id = 1;

    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 1, Some(dec!(20))));
    bursar.process_transaction(Transaction::new(
        Op::Withdrawal,
        client_id,
        2,
        Some(dec!(10)),
    ));

    let client = bursar.clients.get(&client_id);
    assert!(client.is_some());
    let client = client.unwrap();
    assert_eq!(client.total(), dec!(10));
    assert_eq!(client.available, dec!(10));
    assert_eq!(client.held, dec!(0));
    assert_eq!(client.locked, false);
}

#[test]
fn basic_dispute() {
    let mut bursar = Bursar::new();
    let client_id = 1;

    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 1, Some(dec!(10))));
    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 2, Some(dec!(42))));
    bursar.process_transaction(Transaction::new(Op::Dispute, client_id, 1, None));

    let client = bursar.clients.get(&client_id);
    assert!(client.is_some());
    let client = client.unwrap();
    assert_eq!(client.total(), dec!(52));
    assert_eq!(client.available, dec!(42));
    assert_eq!(client.held, dec!(10));
    assert_eq!(client.locked, false);
}

#[test]
fn resolve_dispute() {
    let mut bursar = Bursar::new();
    let client_id = 1;

    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 1, Some(dec!(10))));
    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 2, Some(dec!(42))));
    bursar.process_transaction(Transaction::new(Op::Dispute, client_id, 1, None));
    bursar.process_transaction(Transaction::new(Op::Resolve, client_id, 1, None));

    let client = bursar.clients.get(&client_id);
    assert!(client.is_some());
    let client = client.unwrap();
    assert_eq!(client.total(), dec!(52));
    assert_eq!(client.available, dec!(52));
    assert_eq!(client.held, dec!(0));
    assert_eq!(client.locked, false);
}

#[test]
fn chargeback_dispute() {
    let mut bursar = Bursar::new();
    let client_id = 1;

    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 1, Some(dec!(10))));
    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 2, Some(dec!(42))));
    bursar.process_transaction(Transaction::new(Op::Dispute, client_id, 1, None));
    bursar.process_transaction(Transaction::new(Op::Chargeback, client_id, 1, None));

    let client = bursar.clients.get(&client_id);
    assert!(client.is_some());
    let client = client.unwrap();
    assert_eq!(client.total(), dec!(42));
    assert_eq!(client.available, dec!(42));
    assert_eq!(client.held, dec!(0));
    assert_eq!(client.locked, true);
}

#[test]
fn false_resolve() {
    let mut bursar = Bursar::new();
    let client_id = 1;

    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 1, Some(dec!(10))));
    bursar.process_transaction(Transaction::new(Op::Deposit, client_id, 2, Some(dec!(42))));
    bursar.process_transaction(Transaction::new(Op::Resolve, client_id, 1, None));

    let client = bursar.clients.get(&client_id);
    assert!(client.is_some());
    let client = client.unwrap();
    assert_eq!(client.total(), dec!(52));
    assert_eq!(client.available, dec!(52));
    assert_eq!(client.held, dec!(0));
    assert_eq!(client.locked, false);
}

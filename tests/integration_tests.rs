use bursar::{Bursar, Op, Transaction};
use rust_decimal::Decimal;

#[test]
fn sanity() {
    let mut bursar = Bursar::new();

    let transactions = vec![
        Transaction::new(Op::Deposit, 1u16, 1u32, Some(Decimal::new(0, 10))),
        Transaction::new(Op::Withdrawal, 1u16, 1u32, Some(Decimal::new(0, 10))),
    ];

    bursar.consume(transactions.into_iter());

    let mut output = Vec::new();
    bursar.write_results(&mut output);

    assert_eq!(output, b"client,available,held,total,locked\n1,0.0000,0.0000,0.0000,false\n");
}

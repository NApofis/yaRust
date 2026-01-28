use bank_system::transaction::TxCombinator;
use bank_system::tx_chain;
use bank_system::Storage;
use bank_system::{Deposit, Transfer, Withdraw};
use bank_system::transaction::Transaction;
use proc_my_macros::say_hello;

fn main() {
    let mut storage = Storage::new();
    storage.add_user("Alice".into());
    storage.add_user("Bob".into());

    let tx = tx_chain!(
        Deposit {
            account: "Alice".into(),
            amount: 500
        },
        Transfer {
            from: "Alice".into(),
            to: "Bob".into(),
            amount: 50
        },
        Withdraw {
            account: "Alice".into(),
            amount: 100
        }
    );

    // Тип переменной `tx` будет таким:
    //
    // TxCombinator<
    //     Deposit,
    //     TxCombinator<
    //         Transfer,
    //         Withdraw
    //     >
    // >
    //
    // То есть макрос раскладывает цепочку транзакций
    // в дерево вложенных TxCombinator'ов.

    println!("Выполняем транзакции через макрос...");
    match tx.apply(&mut storage) {
        Ok(_) => println!("Успешно"),
        Err(e) => println!("Ошибка: {:?}", e),
    }

    println!("Итоговые балансы:");
    for (name, balance) in storage.get_all() {
        println!("{} -> {}", name, balance);
    }

    say_hello!("Привет из процедурного макроса!");

}
use bank_system::Storage;

use std::env;

fn help()
{
    eprintln!("Использование:");
    eprintln!("  add <name> <amount>");
    eprintln!("     Example: add John 200");
    eprintln!("  withdraw <name> <amount>");
    eprintln!("     Example: withdraw John 100");
    eprintln!("  balance <name>");
    eprintln!("     Example: balance John");
}

fn main() {

    let mut storage = Storage::load_data("balance.csv");

    let users = vec!["Jon", "Alice", "Bob", "Vasya"];
    for u in users {
        storage.add_user(u.into());
    }

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        help();
        return;
    }

    match args[1].as_str() {
        "add" => {
            if args.len() != 4 {
                help();
                return;
            }
            let name = args[2].clone();
            let amount = args[3].parse::<i64>().expect("Сумма должна быть числом");
            match storage.deposit(&name, amount) {
                Ok(_) => {
                    println!("Пополнено: {name} на {amount}");
                    storage.save("balance.csv");
                },
                Err(e) => println!("Ошибка: {e}"),
            }
        }
        "withdraw" => {
            if args.len() != 4 {
                help();
                return;
            }
            let name = args[2].clone();
            let amount = args[3].parse::<i64>().expect("Сумма должна быть числом");
            match storage.withdraw(&name, amount) {
                Ok(_) => {
                    println!("Снято: {name} на {amount}");
                    storage.save("balance.csv");
                }
                Err(e) => println!("Ошибка: {e}"),
            }
        }
        "balance" => {
            if args.len() != 3 {
                help();
                return;
            }
            let name = args[2].clone();
            match storage.get_balance(&name) {
                Some(b) => println!("Баланс {name}: {b}"),
                None => println!("Пользователь {name} не найден"),
            }
        }
        _ => {
            help();
        }
    }
}

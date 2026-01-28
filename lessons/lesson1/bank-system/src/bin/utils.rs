use std::io;
use bank_system::{Storage, Deposit, Transfer};
use bank_system::transaction::Transaction;
use std::io::{BufRead, Write};

fn help() {
    println!("=== Bank CLI Utils ===");
    println!("Команды:");
    println!("  add <name> <balance>                            - добавить пользователя");
    println!("  remove <name>                                   - удалить пользователя");
    println!("  deposit <name> <amount>                         - пополнить баланс");
    println!("  withdraw <name> <amount>                        - снять со счёта");
    println!("  balance <name>                                  - показать баланс");
    println!("  transfer <name_from> <name_to> <amount>         - показать баланс");
    println!("  exit                                            - выйти");
}

fn main() {
    let mut storage = Storage::load_data("balance.csv");

    help();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut input = String::new();
        if stdin.lock().read_line(&mut input).unwrap() == 0 {
            break
        }

        let args: Vec<&str> = input.split_whitespace().collect();
        if args.is_empty() {
            continue
        }

        match args[0] {
            "add" => {
                if args.len() != 3 {
                    help();
                    return;
                }
                let name = args[1].to_string();
                let balance: i64 = match args[2].parse() {
                    Ok(b) => b,
                    Err(_) => {
                        println!("Сумма должна быть числом");
                        continue;
                    }
                };
                if storage.add_user(name.clone()).is_some() {
                    let _ = storage.deposit(&name, balance);
                    println!("Пользователь {} добавлен с балансом {}", name, balance);
                    storage.save("balance.csv");
                } else {
                    println!("Пользователь {} уже существует", name);
                }
            }
            "remove" => {
                if args.len() != 2 {
                    help();
                    continue;
                }
                let name = args[1];
                if storage.remove_user(&name.to_string()).is_some() {
                    println!("Пользователь {name} удалён");
                    storage.save("balance.csv");
                } else {
                    println!("Пользователь {name} не найден");
                }
            }
            "deposit" => {
                if args.len() != 3 {
                    help();
                    return;
                }
                let name = args[2].to_string();
                let amount: i64 = match args[2].parse() {
                    Ok(a) => a,
                    Err(_) => {
                        println!("Сумма должна быть числом");
                        continue;
                    }
                };
                let tx = Deposit::new(name.clone(), amount);
                // Применяем транзакцию 
                match tx.apply(&mut storage) {
                    Ok(_) => {
                        println!("Транзакция: депозит {} на {}", name, amount);
                        storage.save("balance.csv");
                    }
                    Err(e) => println!("Ошибка транзакции: {:?}", e),
                }
            }
            "withdraw" => {
                if args.len() != 3 {
                    help();
                    return;
                }
                let name = args[1].to_string();
                let amount = match args[2].parse::<i64>() {
                    Ok(a) => a,
                    Err(_) => {
                        println!("Сумма должна быть числом");
                        continue;
                    }
                };
                match storage.withdraw(&name, amount) {
                    Ok(_) => {
                        println!("С баланса пользователя {name} снято {amount}");
                        storage.save("balance.csv")
                    },
                    Err(e) => println!("Ошибка: {e}"),
                }
            }
            "balance" => {
                if args.len() != 2 {
                    help();
                    return;
                }
                let name = args[1];
                match storage.get_balance(&name.into()) {
                    Some(b) => println!("Баланс {name}: {b}"),
                    None => println!("Пользователь {name} не найден"),
                }
            }
            "transfer" => {
                if args.len() != 3 {
                    help();
                    return;
                }
                let name_from = args[1].to_string();
                let name_to = args[2].to_string();
                let amount: i64 = match args[3].parse::<i64>() {
                    Ok(a) => a,
                    Err(_) => {
                        println!("Сумма должна быть числом");
                        continue;
                    }
                };
                let tx = Transfer::new(name_from.clone(), name_to.clone(), amount);
                match tx.apply(&mut storage) {
                    Ok(_) => {
                        println!("С баланса пользователя {name_from} снято {amount} и переведено {name_to}");
                        storage.save("balance.csv")
                    },
                    Err(e) => println!("Ошибка транзакции: {:?}", e),
                }
            }
            "+" => {
                if args.len() != 8 {
                    println!(
                        "Пример: + deposit Alice 100 transfer Alice Bob 30: cur {}",
                        args.len()
                    );
                    continue;
                }

                let deposit = Deposit::new(
                    args[2].to_string(),
                    args[3].parse().unwrap_or(0),
                );

                let transfer = Transfer::new(
                    args[5].to_string(),
                    args[6].to_string(),
                    args[7].parse().unwrap_or(0),
                );

                // Здесь мы используем оператор +
                let combined_tx = deposit + transfer;

                match combined_tx.apply(&mut storage) {
                    Ok(_) => println!("Транзакции выполнены!"),
                    Err(e) => println!("Ошибка при выполнении: {:?}", e),
                }

                storage.save("balance.csv");
            }
            "exit" => break,
            _ => help()
        }
    }
    println!("Выход из CLI, все изменения сохранены.");
}
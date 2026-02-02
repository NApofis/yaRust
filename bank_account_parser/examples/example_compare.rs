use std::{env, io};
use std::fs::File;
use bank_account_parser;
use bank_account_parser::transactions_holder::TransactionHolder;

fn help() {
    println!("Сравнение транзакций из mt940, camt053 и csv:");
    println!("  compare <path1> <path2>");
    println!("Описание:");
    println!("  <path> - путь до одного из файлов [examples/data/mt940.exmpl, examples/data/camt053.exmpl, examples/data/csv.exmpl]");
    println!("  !!! Приведенные пути актуальны для запуска из корня библиотеки bank_account_parser");
    println!("Пример вызова:");
    println!("   compare examples/data/camt053.exmpl examples/data/csv.exmpl");
}

fn get_holder(path: &String) -> Option<TransactionHolder> {
    let Some(filename) = path.split("/").last() else {
        println!("Не удалось получить название файла из параметра {}", &path);
        return None;
    };

    if filename == "camt053.exmpl" {
        let Ok(file) = File::open(&path) else {
            panic!("Не удалось открыть файл {path}")
        };
        let mut reader = io::BufReader::new(file);

        match bank_account_parser::camt053_format::Camt053Format::from_read(&mut reader) {
            Ok(obj) => {
                return Some(TransactionHolder::new(obj));
            },
            Err(e) => {
                println!("{e}");
                panic!("Непредвиденная ошибка!!!")
            }
        };

    } else if filename == "mt940.exmpl" {
        let Ok(file) = File::open(&path) else {
            panic!("Не удалось открыть файл {path}")
        };
        let mut reader = io::BufReader::new(file);

        match bank_account_parser::mt940_format::MT940Format::from_read(&mut reader) {
            Ok(obj) => {
                return Some(TransactionHolder::new(obj));
            },
            Err(e) => {
                println!("{e}");
                panic!("Непредвиденная ошибка!!!")
            }
        };
    } else if filename == "csv.exmpl" {
        let Ok(file) = File::open(&path) else {
            panic!("Не удалось открыть файл {path}")
        };
        let mut reader = io::BufReader::new(file);

        match bank_account_parser::csv_format::CSVFormat::from_read(&mut reader) {
            Ok(obj) => {
                return Some(TransactionHolder::new(obj));
            },
            Err(e) => {
                println!("{e}");
                panic!("Непредвиденная ошибка!!!")
            }
        };
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // println!("{:?}", args);
    if args.len() != 4 || args[1] != "compare" {
        help();
        return;
    }

    let Some(holder1) = get_holder(&args[2]) else {
        help();
        return;
    };

    let Some(holder2) = get_holder(&args[3]) else {
        help();
        return;
    };

    let mut iter_holder1 = holder1.into_iter();
    let mut iter_holder2 = holder2.into_iter();

    // Обход транзакций
    loop {
        if let Some(t1) = iter_holder1.next() {
            let Some(t2) = iter_holder2.next() else {
                println!("Транзакция ({}) есть в {} но нет в {}", t1, args[2], args[3]);
                return;
            };
            if t1 == t2 {
                continue;
            }
        } else if let Some(t2) = iter_holder2.next() {
            println!("Транзакция {} есть в {} но нет в {}", t2, args[3], args[2]);
            return;
        }
        else {
            break
        };
    }
    println!("Транзакции идентичны");
}
use bank_account_parser;
use std::{env, io};
use std::fs::File;
use std::io::Write;

fn help() {
    println!("Конвертирование из mt940 в camt053 или наоборот:");
    println!("  converter <path>");
    println!("Описание:");
    println!("  <path> - путь до одного из файлов [examples/data/mt940.exmpl, examples/data/camt053.exmpl]");
    println!("  результат конвертации выводится в стандартный вывод (stdout)");
    println!("  !!! Приведенные пути актуальны для запуска из корня библиотеки bank_account_parser");
    println!("Пример вызова:");
    println!("   converter examples/data/camt053.exmpl");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // println!("{:?}", args);
    if args.len() != 3 || args[1] != "converter" {
        help();
        return;
    }

    let path = args[2].as_str();
    let Some(filename) = args[2].split("/").last() else {
        println!("Не удалось получить название файла из параметра {}", &args[1]);
        help();
        return;
    };

    if filename == "camt053.exmpl" {
        let Ok(file) = File::open(&path) else {
            panic!("Не удалось открыть файл {path}")
        };
        let mut reader = io::BufReader::new(file);

        let obj = match bank_account_parser::camt053_format::Camt053Format::from_read(&mut reader) {
            Ok(mt) => mt,
            Err(e) => {
                println!("{e}");
                panic!("Непредвиденная ошибка!!!")
            }
        };

        let mut result: bank_account_parser::mt940_format::MT940Format = obj.into();
        let mut out = io::stdout();

        match result.write_to(&mut out) {
            Ok(_) => {
                out.write("\n".as_ref()).unwrap();
            },
            Err(e) => {
                println!("{e}");
                panic!("Непредвиденная ошибка!!!")
            }
        }
    } else if filename == "mt940.exmpl" {
        let Ok(file) = File::open(&path) else {
            panic!("Не удалось открыть файл {path}")
        };
        let mut reader = io::BufReader::new(file);

        let obj = match bank_account_parser::mt940_format::MT940Format::from_read(&mut reader) {
            Ok(mt) => mt,
            Err(e) => {
                println!("{e}");
                panic!("Непредвиденная ошибка!!!")
            }
        };

        let mut result: bank_account_parser::camt053_format::Camt053Format = obj.into();
        let mut out = io::stdout();

        match result.write_to(&mut out) {
            Ok(_) => {
                out.write("\n".as_ref()).unwrap();
            },
            Err(e) => {
                println!("{e}");
                panic!("Непредвиденная ошибка!!!")
            }
        }
    }
    else {
        help();
    }
}
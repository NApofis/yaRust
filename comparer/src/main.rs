use std::{io};
use std::fs::File;
use std::path::PathBuf;

use anyhow::{bail, Result};
use bank_account_parser::camt053_format::Camt053Format;
use bank_account_parser::csv_format::CSVFormat;
use bank_account_parser::mt940_format::MT940Format;
use bank_account_parser::transactions_holder::TransactionHolder;
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InputFormat {
    Mt940,
    Camt053,
    CSV
}

#[derive(Debug, Parser)]
#[command(
    name = "comparer",
    version,
    about = "Bank statement converter (prints result to stdout)"
)]
struct Cli {
    #[arg(long)]
    file1: PathBuf,
    #[arg(long, value_enum)]
    file1_format: InputFormat,

    #[arg(long)]
    file2: PathBuf,
    #[arg(long, value_enum)]
    file2_format: InputFormat,
}


fn main() -> Result<()> {
    let cli = Cli::parse();

    let get_holder = |f: InputFormat, b: &PathBuf| -> Result<TransactionHolder> {
        let res = match f {
            InputFormat::Mt940 => holder_4_mt940(b)?,
            InputFormat::Camt053 => holder_4_camt053(b)?,
            InputFormat::CSV => holder_4_csv(b)?,
        };
        Ok(res)
    };

    let holder1 = get_holder(cli.file1_format, &cli.file1)?;
    let mut iter_holder1 = holder1.into_iter();

    let holder2= get_holder(cli.file2_format, &cli.file2)?;
    let mut iter_holder2 = holder2.into_iter();

    loop {
        if let Some(t1) = iter_holder1.next() {
            let Some(t2) = iter_holder2.next() else {
                bail!("Транзакция ({}) есть в {} но нет в {}", t1, cli.file1.display(), cli.file2.display());
            };
            if t1 == t2 {
                continue;
            }
        } else if let Some(t2) = iter_holder2.next() {
            bail!("Транзакция {} есть в {} но нет в {}", t2, cli.file2.display(), cli.file1.display())
        }
        else {
            break
        };
    }
    println!("Транзакции идентичны");
    Ok(())
}

fn holder_4_mt940(input: &PathBuf) -> Result<TransactionHolder> {

    let Ok(file) = File::open(input) else {
        bail!("Не удалось открыть файл {}", input.display());
    };

    let mut reader = io::BufReader::new(file);

    let obj = match MT940Format::from_read(&mut reader) {
        Ok(o) => o,
        Err(e) => bail!(e.to_string())
    };

    Ok(TransactionHolder::new(obj))
}

fn holder_4_camt053(input: &PathBuf) -> Result<TransactionHolder> {
    let Ok(file) = File::open(input) else {
        bail!("Не удалось открыть файл {}", input.display());
    };

    let mut reader = io::BufReader::new(file);

    let obj = match Camt053Format::from_read(&mut reader) {
        Ok(o) => o,
        Err(e) => bail!(e.to_string())
    };

    Ok(TransactionHolder::new(obj))
}

fn holder_4_csv(input: &PathBuf) -> Result<TransactionHolder> {
    let Ok(file) = File::open(input) else {
        bail!("Не удалось открыть файл {}", input.display());
    };

    let mut reader = io::BufReader::new(file);

    let obj = match CSVFormat::from_read(&mut reader) {
        Ok(o) => o,
        Err(e) => bail!(e.to_string())
    };

    Ok(TransactionHolder::new(obj))
}

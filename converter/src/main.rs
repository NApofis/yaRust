use std::{io};
use std::fs::File;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};

use bank_account_parser::camt053_format::Camt053Format;
use bank_account_parser::mt940_format::MT940Format;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InputFormat {
    Mt940,
    Camt053,
}

#[derive(Debug, Parser)]
#[command(
    name = "converter",
    version,
    about = "Bank statement converter (prints result to stdout)"
)]
struct Cli {
    #[arg(long)]
    input: PathBuf,

    #[arg(long, value_enum)]
    input_format: InputFormat,
}


fn main() -> Result<()> {
    let cli = Cli::parse();

    let output = match cli.input_format {
        InputFormat::Mt940 => convert_mt940(&cli.input),
        InputFormat::Camt053 => convert_camt053(&cli.input),
    };
    let Ok(output) = output else {
        bail!("Не удалось выполнить конвертацию форматов");
    };
    println!("\n{output}");
    Ok(())
}

fn convert_mt940(input: &PathBuf) -> Result<String> {
    let Ok(file) = File::open(input) else {
        bail!("Не удалось открыть файл {}", input.display())
    };

    let mut reader = io::BufReader::new(file);

    let mt = match MT940Format::from_read(&mut reader) {
        Ok(c) => c,
        Err(e) => bail!(e.to_string())
    };

    let mut camt: Camt053Format = mt.into();
    let mut out = io::stdout();

    match camt.write_to(&mut out) {
        Ok(_) => (),
        Err(e) => bail!(e.to_string())
    }

    Ok(format!("Mt940({}) конвертирован в Camt053", input.display()))
}

fn convert_camt053(input: &PathBuf) -> Result<String> {

    let Ok(file) = File::open(input) else {
        bail!("Не удалось открыть файл {}", input.display())
    };

    let mut reader = io::BufReader::new(file);

    let camt = match Camt053Format::from_read(&mut reader) {
        Ok(mt) => mt,
        Err(e) => bail!(e.to_string())
    };

    let mut mt: MT940Format = camt.into();
    let mut out = io::stdout();

    match mt.write_to(&mut out) {
        Ok(_) => (),
        Err(e) => bail!(e.to_string())
    }

    Ok(format!("Camt053({}) конвертирован в Mt940", input.display()))
}
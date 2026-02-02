use std::str::FromStr;
use std::fmt;
use std::fmt::Formatter;
use crate::common::debit_credit::DebitOrCredit;

#[derive(Debug, Eq, PartialEq)]
pub enum FormatError {
    DataFormatError(String),
    UnknownValueFormat(String),
    UnknownError(String),
    ReadWriteError(String),
    UnsupportedTag(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let result = match self {
            FormatError::DataFormatError(s) => s,
            FormatError::UnknownValueFormat(s) => s,
            FormatError::UnknownError(s) => s,
            FormatError::ReadWriteError(s) => s,
            FormatError::UnsupportedTag(s) => s,
        };
        write!(f, "{}", result)
    }
}

impl From<std::io::Error> for FormatError {
    fn from(error: std::io::Error) -> Self {
        FormatError::ReadWriteError(format!("Ошибка чтения/записии, {}", error))
    }
}

pub trait GeneratorFormatError {
    const ERROR_PREFIX: &'static str;

    // Конкретные методы для каждого типа ошибки
    fn data_format_error(details: &str) -> FormatError
    where
        Self: Sized,
    {
        FormatError::DataFormatError(format!("{} : {}", Self::ERROR_PREFIX, details))
    }

    fn unsupported_tag_error(details: &str) -> FormatError
    where
        Self: Sized,
    {
        FormatError::UnsupportedTag(format!("{} : {}", Self::ERROR_PREFIX, details))
    }

    fn unknown_value_error(details: &str) -> FormatError
    where
        Self: Sized,
    {
        FormatError::UnknownValueFormat(format!("{} : {}", Self::ERROR_PREFIX, details))
    }

    fn read_write_error(details: &str) -> FormatError
    where
        Self: Sized,
    {
        FormatError::ReadWriteError(format!("{} : {}", Self::ERROR_PREFIX, details))
    }

    fn unknown_error(details: &str) -> FormatError
    where
        Self: Sized,
    {
        FormatError::UnknownError(format!("{} : {}", Self::ERROR_PREFIX, details))
    }
}

pub mod debit_credit {
    #[derive(Debug, Eq, PartialEq, Default, Copy, Clone)]
    pub enum DebitOrCredit {
        #[default]
        Debit,
        Credit,
        ReverseDebit,
        ReverseCredit,
    }

    impl DebitOrCredit {
        pub fn to_string(&self) -> &str {
            match self {
                DebitOrCredit::Debit => "D",
                DebitOrCredit::Credit => "C",
                DebitOrCredit::ReverseDebit => "RC",
                DebitOrCredit::ReverseCredit => "RD",
            }
        }
    }
}


impl FromStr for DebitOrCredit {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dc = if s == "C" {
            DebitOrCredit::Credit
        } else if s == "D" {
            DebitOrCredit::Debit
        } else if s == "RD" {
            DebitOrCredit::ReverseCredit
        } else if s == "RC" {
            DebitOrCredit::ReverseDebit
        } else {
            return Err(FormatError::UnknownValueFormat(format!(
                "Неизвестное обозначение \"{}\" для операции mt940",
                s
            )));
        };
        Ok(dc)
    }
}

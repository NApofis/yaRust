use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
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

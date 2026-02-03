use std::str::FromStr;
use crate::error::FormatError;
use crate::common::debit_credit::DebitOrCredit;

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

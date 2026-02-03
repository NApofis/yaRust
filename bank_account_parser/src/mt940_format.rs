use chrono::{Datelike, NaiveDate};
use mt940::{Field, ParseError, parse_fields};
use regex::Regex;
use rust_decimal::Decimal;
use std::io::{BufRead, BufReader, Read, Write};
use std::mem;
use std::str::FromStr;
use rust_decimal::prelude::Zero;
use crate::camt053_format::Camt053Format;
use crate::error::{FormatError, GeneratorFormatError};
use crate::common::debit_credit::DebitOrCredit;
use crate::transactions_holder::{Transaction, TransactionsReader};

impl From<ParseError> for FormatError {
    fn from(error: ParseError) -> Self {
        const MT940_ERROR: &str = "Ошибка разбора формата mt940";
        match error {
            ParseError::PestParseError(e) => FormatError::DataFormatError(format!(
                "{} : некорректный формат данных. {}",
                MT940_ERROR,
                e
            )),
            ParseError::UnexpectedTagError(e) => FormatError::DataFormatError(format!(
                "{} : некорректный формат данных. {}",
                MT940_ERROR,
                e
            )),
            ParseError::DateParseError(e) => FormatError::UnknownValueFormat(format!(
                "{} : не удалось разобрать одно из значений. {}",
                MT940_ERROR,
                e
            )),
            ParseError::RequiredTagNotFoundError(e) => FormatError::DataFormatError(format!(
                "{} : некорректный формат данных. {}",
                MT940_ERROR,
                e
            )),
            ParseError::UnknownTagError(e) => FormatError::DataFormatError(format!(
                "{} : некорректный формат данных. {}",
                MT940_ERROR,
                e
            )),
            ParseError::UnknownSubfieldError(e) => FormatError::DataFormatError(format!(
                "{} : некорректный формат данных. {}",
                MT940_ERROR,
                e
            )),
            ParseError::VariantNotFound(e) => FormatError::UnknownValueFormat(format!(
                "{} : не удалось разобрать одно из значений. {}",
                MT940_ERROR,
                e
            )),
            ParseError::AmountParseError(e) => FormatError::UnknownValueFormat(format!(
                "{} : не удалось разобрать одно из значений. {}",
                MT940_ERROR,
                e
            )),
        }
    }
}

#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct AvailableBalance {
    pub debit_credit_indicator: DebitOrCredit,
    pub date: NaiveDate,
    pub iso_currency_code: String,
    pub amount: Decimal,
}

impl AvailableBalance {
    pub fn merge(&mut self, balance: &AvailableBalance) {
        if balance.debit_credit_indicator != DebitOrCredit::Debit {
            self.debit_credit_indicator = balance.debit_credit_indicator;
        }
        if balance.date != NaiveDate::default() {
            self.date = balance.date;
        }
        if !balance.iso_currency_code.is_empty() {
            self.iso_currency_code = balance.iso_currency_code.clone();
        }
        if balance.amount != Decimal::zero() {
            self.amount = balance.amount;
        }
    }
}

#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct Balance {
    pub is_intermediate: bool,
    pub balance: AvailableBalance,
}

impl Balance {
    pub fn merge(&mut self, balance: &Balance) {
        if balance.is_intermediate {
            self.is_intermediate = true;
        }
        self.balance.merge(&balance.balance);
    }
}

impl From<Balance> for AvailableBalance {
    fn from(value: Balance) -> Self {
        value.balance
    }
}

#[derive(Default, Eq, PartialEq, Debug)]
pub struct StatementLine {
    pub value_date: NaiveDate,
    pub entry_date: Option<NaiveDate>,
    pub ext_debit_credit_indicator: DebitOrCredit,
    pub funds_code: Option<String>,
    pub amount: Decimal,
    pub transaction_type_ident_code: String,
    pub customer_ref: String,
    pub bank_ref: Option<String>,
    pub supplementary_details: Option<String>,
    pub information_to_account_owner: Option<String>,
}

#[derive(Default, Eq, PartialEq)]
pub struct Message {
    pub transaction_ref_no: String,
    pub ref_to_related_msg: Option<String>,
    pub account_id: String,
    pub statement_no: String,
    pub sequence_no: Option<String>,
    pub opening_balance: Balance,
    pub statement_lines: Vec<StatementLine>,
    pub closing_balance: Balance,
    pub closing_available_balance: Option<AvailableBalance>,
    pub forward_available_balance: Option<AvailableBalance>,
    pub information_to_account_owner: Option<String>,
}

#[derive(PartialEq)]
enum ReadingState {
    Empty,
    Accumulate,
    Ready,
}

#[derive(Default)]
pub struct MT940Format {
    pub(crate) transactions: Vec<Message>,
    other_data: Vec<String>,
}

impl GeneratorFormatError for MT940Format {
    const ERROR_PREFIX: &'static str = "Ошибка разбора формата mt940";
}

impl MT940Format {
    fn parse_block4(statement: &str) -> Result<Vec<Message>, FormatError> {
        let fields: Vec<Field> = parse_fields(statement).map_err(|e| {
            Self::unknown_value_error(format!("не удалось разбить строку на блоки. {}", e).as_str())
        })?;

        let mut messages: Vec<Message> = Vec::new();
        let mut cur: Option<Message> = None;

        for f in fields {
            let tag = f.tag.as_str();
            let value = f.value;

            match tag {
                "20" => {
                    if let Some(m) = cur.take() {
                        messages.push(m);
                    }
                    cur = Some(Message {
                        transaction_ref_no: value.to_string(),
                        ..Default::default()
                    });
                }
                "21" => {
                    let m = cur
                        .as_mut()
                        .ok_or_else(|| Self::data_format_error("найден блок 21 без блока 20"))?;
                    m.ref_to_related_msg = Some(value);
                }
                "25" => {
                    let m = cur
                        .as_mut()
                        .ok_or_else(|| Self::data_format_error("найден блок 25 без блока 20"))?;
                    m.account_id = value;
                }
                "28" | "28C" => {
                    let m = cur.as_mut().ok_or_else(|| {
                        Self::data_format_error("найден блок 28/28C без блока 20")
                    })?;
                    if let Some((a, b)) = value.split_once('/') {
                        m.statement_no = a.to_string();
                        m.sequence_no = Some(b.to_string());
                    } else {
                        m.statement_no = value;
                        m.sequence_no = None;
                    }
                }
                "60F" | "60M" => {
                    let m = cur
                        .as_mut()
                        .ok_or_else(|| Self::data_format_error("найден блок 60 без блока 20"))?;
                    m.opening_balance = Self::parse_balance(&value, 11, tag.ends_with('M'))?;
                }
                "61" => {
                    let m = cur
                        .as_mut()
                        .ok_or_else(|| Self::data_format_error("найден блок 61 без блока 20"))?;
                    let stmt = Self::parse_61(&value)?;
                    m.statement_lines.push(stmt);
                }
                "86" => {
                    let m = cur
                        .as_mut()
                        .ok_or_else(|| Self::data_format_error("найден блок 86 без блока 20"))?;
                    if let Some(last) = m.statement_lines.last_mut() {
                        // 86 относится к последней 61
                        last.information_to_account_owner = Some(value);
                    } else {
                        // fallback: если вдруг 86 идёт на уровне statement (редко/грязные данные)
                        m.information_to_account_owner = Some(value);
                    }
                }
                "62F" | "62M" => {
                    let m = cur
                        .as_mut()
                        .ok_or_else(|| Self::data_format_error("найден блок 62 без блока 20"))?;
                    m.closing_balance = Self::parse_balance(&value, 11, tag.ends_with('M'))?;
                }
                "64" => {
                    let m = cur
                        .as_mut()
                        .ok_or_else(|| Self::data_format_error("найден блок 64 без блока 20"))?;
                    m.closing_available_balance =
                        Some(Self::parse_balance(&value, 13, false)?.into());
                }
                "65" => {
                    let m = cur
                        .as_mut()
                        .ok_or_else(|| Self::data_format_error("найден блок 65 без блока 20"))?;
                    m.forward_available_balance =
                        Some(Self::parse_balance(&value, 13, false)?.into());
                }
                _ => {
                    return Err(Self::unsupported_tag_error(
                        "неизвестный или недопустимый тег",
                    ));
                }
            }
        }

        if let Some(m) = cur.take() {
            messages.push(m);
        }

        Ok(messages)
    }

    fn parse_balance(s: &str, size: usize, is_intermediate: bool) -> Result<Balance, FormatError> {
        // <C/D><YYMMDD><CUR><AMOUNT> - balance - 11
        // <C/D><YYYYMMDD><CUR><AMOUNT> - available_balance - 13

        let s = s.trim();
        if s.len() < size {
            return Err(Self::unknown_value_error(
                format!("слишком короткий баланс {}", s).as_str(),
            ));
        }

        let ext_dc = DebitOrCredit::from_str(&s[0..1])?;

        let date = NaiveDate::parse_from_str(&s[1..7], "%y%m%d").map_err(|e| {
            Self::unknown_value_error(
                format!("не удалось разобрать дату баланса {}", e).as_str(),
            )
        })?;

        let cur = s[7..10].to_string();
        let amount_str = s[10..].trim();

        let amount = amount_str.replace(",", ".").parse().map_err(|_| {
            Self::unknown_value_error(
                format!(
                    "не удалось разобрать сумму баланса {}",
                    amount_str
                )
                .as_str(),
            )
        })?;

        Ok(Balance {
            is_intermediate,
            balance: AvailableBalance {
                debit_credit_indicator: ext_dc,
                date,
                iso_currency_code: cur,
                amount
            }
        })
    }

    fn parse_61(raw: &str) -> Result<StatementLine, FormatError> {
        let s = raw.replace('\n', "");
        let s = s.trim();

        if s.len() < 6 {
            return Err(Self::unknown_value_error(
                format!("в блоке 61 нет value_date - {}", s).as_str(),
            ));
        }
        let value_date = NaiveDate::parse_from_str(&s[0..6], "%y%m%d").map_err(|_| {
            Self::unknown_value_error(
                format!("в блоке 61 не удалось разобрать value_date - {}", s).as_str(),
            )
        })?;
        let mut i = 6;

        let entry_date: Option<NaiveDate> =
            if s.len() >= i + 4 && s[i..i + 4].chars().all(|c| c.is_ascii_digit()) {
                let mmdd = &s[i..i + 4];
                i += 4;
                let yyyy = value_date.year();
                NaiveDate::parse_from_str(&format!("{yyyy}{mmdd}"), "%Y%m%d").ok()
            } else {
                None
            };

        // D/C or RD/RC
        let ext_dc: DebitOrCredit = if s.len() >= i + 2 {
            match &s[i..i + 2] {
                "RD" => {
                    i += 2;
                    DebitOrCredit::ReverseCredit
                }
                "RC" => {
                    i += 2;
                    DebitOrCredit::ReverseDebit
                }
                _ => {
                    let one = &s[i..i + 1];
                    i += 1;
                    DebitOrCredit::from_str(one)?
                }
            }
        } else {
            return Err(Self::unknown_value_error(
                format!("в блоке 61 нет D/C - {}", s).as_str(),
            ));
        };

        let funds_code: Option<String> = if s.len() > i {
            let c = s.as_bytes()[i] as char;
            if c.is_ascii_alphabetic()
                && s.len() > i + 1
                && (s.as_bytes()[i + 1] as char).is_ascii_digit()
            {
                i += 1;
                Some(c.to_string())
            } else {
                None
            }
        } else {
            None
        };

        let start_amount = i;
        while i < s.len() && (s.as_bytes()[i] as char).is_ascii_digit() {
            i += 1;
        }
        if i == start_amount || i >= s.len() || (s.as_bytes()[i] != b',' && s.as_bytes()[i] != b'.') {
            return Err(Self::unknown_value_error(
                format!("в блоке 61 не удалось выделить amount - {}", s).as_str(),
            ));
        }
        i += 1;
        let frac_start = i;
        while i < s.len() && (s.as_bytes()[i] as char).is_ascii_digit() {
            i += 1;
        }
        if i == frac_start {
            return Err(Self::unknown_value_error(
                format!(
                    "в блоке 61 не удалось выделить дробную часть amount - {}",
                    s
                )
                .as_str(),
            ));
        }

        let amount_str = &s[start_amount..i];
        let amount = amount_str.replace(",", ".").parse().map_err(|_| {
            Self::unknown_value_error(
                format!("в блоке 61 не удалось разобрать amount - {}", amount_str).as_str(),
            )
        })?;

        // transaction type: N + 3 chars
        if s.len() < i + 4 || &s[i..i + 1] != "N" {
            return Err(Self::unknown_value_error(
                format!("в блоке 61 нет transaction type (NXXX) - {}", s).as_str(),
            ));
        }
        let code3 = &s[i + 1..i + 4];

        i += 4;

        let tail = &s[i..];

        let (customer_ref, bank_ref, supplementary_details) =
            if let Some((left, right)) = tail.split_once("//") {
                if right.len() > 16 {
                    (
                        left.to_string(),
                        Some(right[..16].to_string()),
                        Some(right[16..].to_string()),
                    )
                } else {
                    (left.to_string(), Some(right.to_string()), None)
                }
            } else if tail.len() > 16 {
                (tail[..16].to_string(), None, Some(tail[16..].to_string()))
            } else {
                (tail.to_string(), None, None)
            };

        Ok(StatementLine {
            value_date,
            entry_date,
            ext_debit_credit_indicator: ext_dc,
            funds_code,
            amount,
            transaction_type_ident_code: code3.to_string(),
            customer_ref,
            bank_ref,
            supplementary_details,
            information_to_account_owner: None,
        })
    }

    /// Читает MT940 из произвольного `Read` и возвращает разобранный формат.
    ///
    /// Парсер извлекает блоки `{4: ... -}` (Block 4) из входного потока, сохраняет
    /// «прочие данные» (всё, что находится вне блоков 4), а затем разбирает теги MT940.
    pub fn from_read<R: Read>(r: &mut R) -> Result<Self, FormatError> {
        let reader = BufReader::new(r);
        let mut accum = String::new();
        let mut state = ReadingState::Empty;
        let Ok(start_mt940) = Regex::new(r"}[{ ]*4:") else {
            Err(Self::unknown_error("Не удалось создать Regex"))?
        };
        let Ok(end_mt940) = Regex::new(r"-[)}]|}") else {
            Err(Self::unknown_error("Не удалось создать Regex"))?
        };

        let mut transactions: Vec<Message> = Vec::new();
        let mut other_data: Vec<String> = Vec::new();

        for line in reader.lines().map_while(Result::ok) {
            if let Some(s) = start_mt940.find(&line) {
                if let Some(l) = other_data.last_mut() {
                    l.push_str(line.as_str());
                }
                else {
                    other_data.push(line[..s.start()].to_string());
                }
                accum += &line[s.end()..];
                accum += "\n";
                state = ReadingState::Accumulate;
            } else if state == ReadingState::Accumulate {
                if let Some(e) = end_mt940.find(&line) {
                    other_data.push(line[e.end()..].to_string());
                    accum += &line[..e.start()];
                    state = ReadingState::Ready;
                } else {
                    accum += line.as_str();
                    accum += "\n";
                }
            } else if let Some(l) = other_data.last_mut() {
                l.push_str(line.as_str());
            }
            else {
                other_data.push(line.to_string());
            }

            if state == ReadingState::Ready {
                match Self::parse_block4(&accum) {
                    Ok(e) => transactions.extend(e),
                    Err(e) => return Err(e)?,
                }

                accum.clear();
                state = ReadingState::Empty;
            }
        }
        Ok(Self {
            transactions,
            other_data,
        })
    }

    fn write_message<W: Write>(writer: &mut W, tag: &str, value: &str, first: &mut bool) -> Result<(), FormatError> {
        if *first {
            writer.write_all("{4:\n".as_bytes())?;
            *first = false;
        }
        writer.write_fmt(format_args!(":{}:{}\n", tag, value))?;
        Ok(())
    }

    fn write_balance<W: Write>(writer: &mut W, tag: &str, balance: &Balance, first: &mut bool) -> Result<(), FormatError> {
        if balance.balance.amount.is_zero() {
            return Ok(());
        }

        let current_tag = if balance.is_intermediate {
            tag.to_string() + "M"
        } else {
            tag.to_string() + "F"
        };
        let mut result = String::new();
        result += balance.balance.debit_credit_indicator.to_string();
        result += &balance.balance.date.format("%y%m%d").to_string();
        result += &balance.balance.iso_currency_code;
        result += &balance.balance.amount.to_string();
        Self::write_message(writer, &current_tag, &result, first)?;
        Ok(())
    }

    fn write_statement<W: Write>(writer: &mut W, statement: &StatementLine, first: &mut bool) -> Result<(), FormatError> {
        let mut result = String::new();
        result += &statement.value_date.format("%y%m%d").to_string();
        if let Some(d) = statement.entry_date.as_ref() {
            result += &d.format("%m%d").to_string();
        }
        result += statement.ext_debit_credit_indicator.to_string();
        if let Some(f) = statement.funds_code.as_ref() {
            result += f;
        }
        result += &statement.amount.to_string();
        let val = &statement.transaction_type_ident_code;
        result += &format!("N{val}");
        result += &statement.customer_ref.to_string();
        if let Some(b) = statement.bank_ref.as_ref() {
            result += "//";
            result += b;
        }
        if let Some(s) = statement.supplementary_details.as_ref() {
            result += s;
        }
        Self::write_message(writer, "61", result.as_str(), first)?;
        if let Some(i) = statement.information_to_account_owner.as_ref() {
            Self::write_message(writer, "86", i, first)?;
        }
        Ok(())
    }

    fn write_available_balance<W: Write>(writer: &mut W, tag: &str, balance: &AvailableBalance, first: &mut bool) -> Result<(), FormatError> {
        let mut result = String::new();
        result += balance.debit_credit_indicator.to_string();
        result += &balance.date.format("%Y%m%d").to_string();
        result += &balance.iso_currency_code;
        result += &balance.amount.to_string();
        Self::write_message(writer, tag, &result, first)?;
        Ok(())
    }

    /// Записывает представление MT940 в `writer`.
    ///
    /// Метод формирует блоки `{4: ... -}` по каждому сообщению и добавляет
    /// сохранённые «прочие данные» (префиксы/суффиксы, встреченные при разборе).
    pub fn write_to<W: Write>(&mut self, writer: &mut W) -> Result<(), FormatError> {
        for (index, message) in self.transactions.iter().enumerate() {
            let mut first_write = true;

            if index < self.other_data.len() {
                writer.write_all(self.other_data[index].to_string().as_bytes())?;
            }
            if !message.transaction_ref_no.is_empty() {
                Self::write_message(writer, "20", &message.transaction_ref_no, &mut first_write)?;
            }

            if let Some(x) = message.ref_to_related_msg.as_ref() {
                Self::write_message(writer, "21", x.as_str(), &mut first_write)?;
            }

            if !message.account_id.is_empty() {
                Self::write_message(writer, "25", &message.account_id, &mut first_write)?;
            }

            if !message.statement_no.is_empty() {
                let mut value = message.statement_no.clone();
                if let Some(v) = message.sequence_no.as_ref() {
                    value += "/";
                    value.push_str(v);
                }
                Self::write_message(writer, "28C", value.as_str(), &mut first_write)?;
            }

            Self::write_balance(writer, "60", &message.opening_balance, &mut first_write)?;
            for transaction in &message.statement_lines {
                Self::write_statement(writer, transaction, &mut first_write)?;
            }
            Self::write_balance(writer, "62", &message.closing_balance, &mut first_write)?;
            if let Some(a) = message.closing_available_balance.as_ref() {
                Self::write_available_balance(writer, "64", a, &mut first_write)?;
            }
            if let Some(a) = message.forward_available_balance.as_ref() {
                Self::write_available_balance(writer, "65", a, &mut first_write)?;
            }
            if let Some(i) = message.information_to_account_owner.as_ref() {
                Self::write_message(writer, "86", i, &mut first_write)?;
            }

            if !first_write {
                writer.write_all("-}".as_bytes())?;
            }
        }
        if self.other_data.len() > self.transactions.len() && let Some(l) = self.other_data.last() {
            writer.write_all(l.to_string().as_bytes())?;
        }

        Ok(())
    }

    fn get_avalbal<'a>(name: &String, message: &'a mut Message) -> Option<&'a mut AvailableBalance>{
        match name.as_str() {
            "CLBD" => Some(&mut message.closing_balance.balance),
            "CLAV" => {
                message.closing_available_balance = Some(AvailableBalance::default());
                message.closing_available_balance.as_mut()
            },
            "FWAV" => {
                message.forward_available_balance = Some(AvailableBalance::default());
                message.forward_available_balance.as_mut()
            },
            "OPBD" | _ => Some(&mut message.opening_balance.balance),
        }
    }

}

impl<'a> IntoIterator for &'a MT940Format {
    type Item = &'a Message;
    type IntoIter = std::slice::Iter<'a, Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.transactions.iter()
    }
}

impl From<Camt053Format> for MT940Format {
    fn from(value: Camt053Format) -> Self {

        let mut result = Vec::new();
        let mut message = Message::default();
        let mut base_orgn_msg_id = String::new();

        let mut balance = Balance::default();
        let mut balance_name = String::new();

        let mut statement: Option<StatementLine> = None;

        for tag in value.get_iter() {
            let path = tag.path();
            let Some(s) = path.find("/Stmt") else {
                if tag.path().as_str() == "/BkToCstmrStmt/GrpHdr/OrgnlBizQry/MsgId" {
                    base_orgn_msg_id = tag.text()
                };
                continue;
            };

            match &path[s..] {
                "/Stmt" => {
                    if !message.transaction_ref_no.is_empty() {
                        if !base_orgn_msg_id.is_empty() {
                            message.ref_to_related_msg = Some(base_orgn_msg_id.clone());
                        }
                        result.push(mem::take(&mut message));
                    }
                    else if !base_orgn_msg_id.is_empty() {
                        message.ref_to_related_msg = Some(base_orgn_msg_id.clone());
                    }
                }
                "/Stmt/Id" => message.transaction_ref_no = tag.text(),
                "/Stmt/Acct/Id/IBAN" | "Stmt/Acct/Id/Othr/Id" => message.account_id = tag.text(),
                "/Stmt/ElctrncSeqNb" => message.statement_no = tag.text(),
                "/Stmt/LglSeqNb" => {
                    if !tag.text().is_empty() {
                        message.sequence_no = Some(tag.text());
                    }
                }
                "/Stmt/AddtlStmtInf" => {
                    if !tag.text().is_empty() {
                        message.information_to_account_owner = Some(tag.text());
                    }
                }
                "/Stmt/Bal" => {
                    if !balance_name.is_empty() && let Some(a) = Self::get_avalbal(&balance_name, &mut message) {
                        a.merge(&balance.balance);
                    }
                    balance = Balance::default();
                    balance_name.clear();
                }
                "/Stmt/Bal/Tp/CdOrPrtry/Cd" => {
                    balance_name = tag.text();
                }
                "/Stmt/Bal/CdtDbtInd" => {

                    let _type = match tag.text().as_str() {
                        "DBIT" => DebitOrCredit::Debit,
                        "CRDT" => DebitOrCredit::Credit,
                        _ => DebitOrCredit::Debit,
                    };
                    balance.balance.debit_credit_indicator = _type;
                },
                "/Stmt/Bal/Dt/Dt" => {
                    if let Ok(d) = NaiveDate::parse_from_str(tag.text().as_str(), "%Y-%m-%d") {
                        balance.balance.date = d;
                    }
                }
                "/Stmt/Bal/Amt" => {
                    if let Ok(amount) = tag.text().replace(",", ".").parse() {
                        balance.balance.amount = amount;
                    }
                    if let Some(curr) = tag.get_attr("Ccy") {
                        balance.balance.iso_currency_code = curr;
                    }
                }
                "/Stmt/Ntry" => {
                    if let Some(c) = &mut statement {
                        message.statement_lines.push(mem::take(c));
                    }
                    else {
                        statement = Some(StatementLine::default());
                    }
                }
                "/Stmt/Ntry/ValDt/Dt" => {
                    if let Some(st) = &mut statement
                        && let Ok(d) = NaiveDate::parse_from_str(tag.text().as_str(), "%Y-%m-%d")
                    {
                        st.value_date = d;
                    }
                }
                "/Stmt/Ntry/BookgDt/Dt" => {
                    if let Some(st) = &mut statement
                        && let Ok(d) = NaiveDate::parse_from_str(tag.text().as_str(), "%Y-%m-%d")
                    {
                        st.entry_date = Some(d);
                    }
                }
                "/Stmt/Ntry/CdtDbtInd" => {
                    if let Some(st) = &mut statement {
                        match tag.text().as_str() {
                            "DBIT" => st.ext_debit_credit_indicator = DebitOrCredit::Debit,
                            "CRDT" => st.ext_debit_credit_indicator = DebitOrCredit::Credit,
                            _ => st.ext_debit_credit_indicator = DebitOrCredit::Debit,
                        }
                    }
                }
                "/Stmt/Ntry/Amt" => {
                    if let Some(st) = &mut statement
                        && let Ok(amount) = tag.text().replace(",", ".").parse()
                    {
                        st.amount = amount;
                        st.funds_code = tag.get_attr("Ccy");
                    }
                }
                "/Stmt/Ntry/BkTxCd/Prtry/Issr" => {
                    if let Some(st) = &mut statement {
                        st.transaction_type_ident_code = tag.text();
                    }
                }
                "/Stmt/Ntry/NtryDtls/TxDtls/Refs/EndToEndId"
                | "/Stmt/Ntry/NtryDtls/TxDtls/Refs/MndtId"
                | "/Stmt/Ntry/NtryDtls/TxDtls/Refs/InstrId"
                | "/Stmt/Ntry/NtryDtls/TxDtls/Refs/PmtInfId" => {
                    if let Some(st) = &mut statement {
                        st.customer_ref = tag.text();
                    }
                }
                "/Stmt/Ntry/NtryDtls/TxDtls/Refs/AcctSvcrRef"
                | "/Stmt/Ntry/NtryDtls/TxDtls/Refs/TxId" => {
                    if let Some(st) = &mut statement {
                        st.bank_ref = Some(tag.text());
                    }
                }
                "/Stmt/Ntry/AddtlTxInf" => {
                    if let Some(st) = &mut statement {
                        st.supplementary_details = Some(tag.text());
                    }
                }
                "/Stmt/Ntry/NtryDtls/TxDtls/AddtlTxInf" => {
                    if let Some(st) = &mut statement {
                        if let Some(_exists) = &mut st.information_to_account_owner {
                            _exists.push(' ');
                            _exists.push_str(tag.text().as_str());
                        } else {
                            st.information_to_account_owner = Some(tag.text());
                        }
                    }
                }
                _ => continue,
            }
        }
        if !balance_name.is_empty() && let Some(a) = Self::get_avalbal(&balance_name, &mut message) {
            a.merge(&balance.balance);
        }

        if let Some(c) = &mut statement {
            message.statement_lines.push(mem::take(c));
        }

        if message != Message::default() {
            result.push(mem::take(&mut message));
        }

        Self {
            other_data: vec!["{3:}".into()],
            transactions: result,
        }
    }
}

impl TransactionsReader for MT940Format {
    fn collect_transactions(&self) -> Vec<Transaction> {
        let mut transactions = Vec::new();
        for msg in &self.transactions {
            for statement in &msg.statement_lines {
                transactions.push(Transaction {
                    amount: statement.amount,
                    operation_type: statement.ext_debit_credit_indicator,
                    date: statement.value_date,
                    currency: msg.opening_balance.balance.iso_currency_code.clone()
                });
            }
        }
        transactions
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use crate::camt053_format::Camt053Format;

    #[test]
    fn check_balance_error(){
        let result = MT940Format::parse_balance("C240101", 11, false).unwrap_err();
        assert_eq!(result, FormatError::UnknownValueFormat("Ошибка разбора формата mt940 : слишком короткий баланс C240101".to_string()));

        let result = MT940Format::parse_balance("C--0101USD123", 11, false).unwrap_err();
        assert_eq!(result, FormatError::UnknownValueFormat("Ошибка разбора формата mt940 : не удалось разобрать дату баланса input contains invalid characters".to_string()));

        let result = MT940Format::parse_balance("C240101USD+++", 11, false).unwrap_err();
        assert_eq!(result, FormatError::UnknownValueFormat("Ошибка разбора формата mt940 : не удалось разобрать сумму баланса +++".to_string()));
    }

    #[test]
    fn parse_balance_yy_mm_dd() {
        let b = MT940Format::parse_balance("C240101USD123,45", 11, false).unwrap();
        assert_eq!(b.balance.date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(b.balance.iso_currency_code, "USD");
        assert_eq!(b.balance.amount, Decimal::from_str("123.45").unwrap());
        assert!(!b.is_intermediate);
    }

    #[test]
    fn parse_61_basic() {
        let st = MT940Format::parse_61("2401010101D123,45NTRFREF1//BANKREF0123456789").unwrap();
        assert_eq!(st.value_date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(st.entry_date, Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()));
        assert_eq!(st.amount, Decimal::from_str("123.45").unwrap());
        assert_eq!(st.transaction_type_ident_code, "TRF");
        assert_eq!(st.customer_ref, "REF1");
        assert!(st.bank_ref.as_deref().unwrap().starts_with("BANKREF"));
    }

    #[test]
    fn check_block61_error(){
        let result = MT940Format::parse_61("2401010101D123").unwrap_err();
        assert_eq!(result, FormatError::UnknownValueFormat("Ошибка разбора формата mt940 : в блоке 61 не удалось выделить amount - 2401010101D123".to_string()));

        let result = MT940Format::parse_61("--01010101D123,45NTRFREF1//BANKREF0123456789").unwrap_err();
        assert_eq!(result, FormatError::UnknownValueFormat("Ошибка разбора формата mt940 : в блоке 61 не удалось разобрать value_date - --01010101D123,45NTRFREF1//BANKREF0123456789".to_string()));

        let result = MT940Format::parse_61("2401010101D1oo11NTRFREF1//BANKREF0123456789").unwrap_err();
        assert!(matches!(result, FormatError::UnknownValueFormat(_)));

        let result = MT940Format::parse_61("2401010101D111TRFREF1//BANKREF0123456789").unwrap_err();
        assert!(matches!(result, FormatError::UnknownValueFormat(_)));
    }

    fn sample_block4() -> String {
        [
            ":20:TRN123456",
            ":25:DE12500105170648489890",
            ":28C:00001/001",
            ":60F:C240101EUR100,00",
            ":61:2401020102D1,23NTRFNONREF//ABC123",
            ":86:TEST PAYMENT",
            ":62F:C240102EUR98,77"
        ].join("\n")
    }

    #[test]
    fn test_parse_block4_ok() {
        let result = MT940Format::parse_block4(":21:TRN123456");
        assert!(matches!(result, Err(FormatError::DataFormatError(_))));

        let input = sample_block4();
        let stmt_vec = MT940Format::parse_block4(&input).expect("parse_block4 должно быть успешным");

        let stmt = stmt_vec.first().unwrap();

        // Минимальные проверки по важным полям.
        assert_eq!(stmt.transaction_ref_no, "TRN123456");
        assert_eq!(stmt.account_id, "DE12500105170648489890");
        assert_eq!(stmt.statement_no, "00001");

        // Opening balance
        assert_eq!(stmt.opening_balance.balance.iso_currency_code, "EUR");
        assert_eq!(stmt.opening_balance.balance.debit_credit_indicator, DebitOrCredit::Credit);
        assert_eq!(stmt.opening_balance.balance.date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(stmt.opening_balance.balance.amount, Decimal::from_str("100.00").unwrap());

        // Closing balance
        assert_eq!(stmt.closing_balance.balance.iso_currency_code, "EUR");
        assert_eq!(stmt.closing_balance.balance.debit_credit_indicator, DebitOrCredit::Credit);
        assert_eq!(stmt.closing_balance.balance.date, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        assert_eq!(stmt.closing_balance.balance.amount, Decimal::from_str("98.77").unwrap());

        // Transactions
        assert_eq!(stmt.statement_lines.len(), 1);

        let tx = &stmt.statement_lines[0];
        assert_eq!(tx.value_date, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        assert_eq!(tx.entry_date, Some(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()));
        assert_eq!(tx.ext_debit_credit_indicator, DebitOrCredit::Debit);
        assert_eq!(tx.amount, Decimal::from_str("1.23").unwrap());
        assert_eq!(tx.transaction_type_ident_code, "TRF");

        assert!(!tx.customer_ref.is_empty());

        // :86:
        assert_eq!(
            tx.information_to_account_owner.as_deref(),
            Some("TEST PAYMENT")
        );
    }
    #[test]
    fn test_from_read_ok() {
        let text = format!(
            "{{1:F01BANKBEBBAXXX0000000000}}{{2:I940BANKDEFFXXXXN}}{{4:\n{}\n-}}",
            sample_block4()
        );

        let mut cur = Cursor::new(text.as_bytes());

        let mt = MT940Format::from_read(&mut cur).expect("from_read must succeed");
        let stmt = mt.transactions.first().unwrap();
        assert_eq!(stmt.transaction_ref_no, "TRN123456");
        assert_eq!(stmt.account_id, "DE12500105170648489890");

        let first_other = mt.other_data.first().unwrap();
        assert!(first_other.contains("F01BANKBEBBAXXX0000000000"));
    }


    fn get_balance() -> Balance{
        Balance {
            is_intermediate: false,
            balance: AvailableBalance {
                debit_credit_indicator: DebitOrCredit::Credit,
                date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                iso_currency_code: "EUR".to_string(),
                amount: Decimal::from_str("123.10").unwrap(),
            }
        }
    }

    #[test]
    fn write_statement_formats_code_and_decimal() {
        let mut s = StatementLine::default();
        s.value_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        s.ext_debit_credit_indicator = DebitOrCredit::Debit;
        s.amount = Decimal::from_str("1.23").unwrap();
        s.transaction_type_ident_code = "TRF".to_string();
        s.customer_ref = "ABC".to_string();

        let mut out = Vec::new();
        let mut first = true;
        MT940Format::write_statement(&mut out, &s, &mut first).unwrap();
        let text = String::from_utf8(out).unwrap();

        assert!(text.contains(":61:"));
        assert!(text.contains("NTRF"));
        assert!(text.contains("1.23"));
    }

    #[test]
    fn write_balance_writes_60f_with_yy_mm_dd_and_starts_block4() {
        let balance = get_balance();

        let mut out = Vec::new();
        let mut first = true;

        MT940Format::write_balance(&mut out, "60", &balance, &mut first).unwrap();

        let text = String::from_utf8(out).unwrap();

        assert!(text.starts_with("{4:\n"));
        assert!(text.contains(":60F:"));
        assert!(text.contains("240102"));
        assert!(text.contains("EUR"));
        assert!(text.contains("123.10"));
        assert!(!first);
    }

    #[test]
    fn write_balance_writes_60m_when_intermediate() {
        let mut balance = get_balance();
        balance.is_intermediate = true;

        let mut out = Vec::new();
        let mut first = true;

        MT940Format::write_balance(&mut out, "60", &balance, &mut first).unwrap();

        let text = String::from_utf8(out).unwrap();
        assert!(text.contains(":60M:"));
        assert!(text.contains("240102"));
        assert!(text.contains("EUR"));
        assert!(text.contains("123.10"));
    }

    fn get_available_balance() -> AvailableBalance {
        AvailableBalance {
            debit_credit_indicator: DebitOrCredit::Credit,
            date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            iso_currency_code: "AZN".to_string(),
            amount: Decimal::from_str("77.70").unwrap(),
        }
    }

    #[test]
    fn write_available_balance_writes_tag_and_yyyy_mm_dd_and_starts_block4() {
        let balance = get_available_balance();

        let mut out = Vec::new();
        let mut first = true;

        // tag выберем 64 (closing available)
        MT940Format::write_available_balance(&mut out, "64", &balance, &mut first).unwrap();

        let text = String::from_utf8(out).unwrap();

        assert!(text.starts_with("{4:\n"));
        assert!(text.contains(":64:"));
        // Дата YYYYMMDD (как в вашем write_available_balance)
        assert!(text.contains("20240102"));
        assert!(text.contains("AZN"));
        assert!(text.contains("77.70"));
        assert!(!first);
    }

    pub fn get_message() -> Message {
        let mut msg = Message::default();
        msg.transaction_ref_no = "TRN123456".to_string();
        msg.account_id = "DE12500105170648489890".to_string();
        msg.statement_no = "00001".to_string();
        msg.sequence_no = Some("001".to_string());

        msg.opening_balance = get_balance();
        msg.closing_balance = get_balance();

        let mut st = StatementLine::default();
        st.value_date = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        st.entry_date = Some(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        st.ext_debit_credit_indicator = DebitOrCredit::Debit;
        st.amount = Decimal::from_str("1.23").unwrap();
        st.transaction_type_ident_code = "TRF".to_string();
        st.customer_ref = "NONREF".to_string();
        st.bank_ref = Some("ABC123".to_string());
        st.supplementary_details = Some("SUP".to_string());
        st.information_to_account_owner = Some("TEST PAYMENT".to_string());
        msg.statement_lines.push(st);

        msg
    }

    #[test]
    fn write_to_writes_full_message_block_and_closes() {
        // Собираем одно сообщение с минимально важным набором полей:
        // 20, 25, 60F, 61 + 86(внутри StatementLine), 62F, 64/65 (optional)
        let mut fmt = MT940Format {
            transactions: vec![get_message()],
            other_data: vec!["".to_string()],
        };

        let mut out = Vec::new();
        fmt.write_to(&mut out).unwrap();

        let text = String::from_utf8(out).unwrap();

        assert!(text.contains("{4:\n"));
        assert!(text.contains(":20:TRN123456"));
        assert!(text.contains(":25:DE12500105170648489890"));
        assert!(text.contains(":60F:"));
        assert!(text.contains(":61:"));
        assert!(text.contains(":86:TEST PAYMENT"));
        assert!(text.contains(":62F:"));
        assert!(text.ends_with("-}"));
    }

    #[test]
    fn collect_transactions_returns_same_data_as_in_mt940format() {
        let mut msg = Message::default();

        let mut st1 = StatementLine::default();
        st1.amount = Decimal::from_str("10.50").unwrap();
        st1.ext_debit_credit_indicator = DebitOrCredit::Debit;
        st1.value_date = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        st1.funds_code = None;

        let mut st2 = StatementLine::default();
        st2.amount = Decimal::from_str("99.99").unwrap();
        st2.ext_debit_credit_indicator = DebitOrCredit::Credit;
        st2.value_date = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        st2.funds_code = None;

        msg.opening_balance = Balance {
            is_intermediate: false,
            balance: AvailableBalance {
                debit_credit_indicator: DebitOrCredit::Debit,
                date: NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                iso_currency_code: "EUR".into(),
                amount: Decimal::from_str("1.23").unwrap(),
            }
        };

        msg.statement_lines.push(st1);
        msg.statement_lines.push(st2);

        let fmt = MT940Format {
            transactions: vec![msg],
            ..Default::default()
        };

        // 2) Вызываем collect_transactions
        let txs = fmt.collect_transactions();

        // 3) Проверяем что кол-во совпало
        assert_eq!(txs.len(), 2);

        // 4) Проверяем полное соответствие данных
        // tx[0] <-> statement_lines[0]
        assert_eq!(txs[0].amount, fmt.transactions[0].statement_lines[0].amount);
        assert_eq!(
            txs[0].operation_type,
            fmt.transactions[0].statement_lines[0].ext_debit_credit_indicator
        );
        assert_eq!(txs[0].date, fmt.transactions[0].statement_lines[0].value_date);
        assert_eq!(txs[0].currency, "EUR");

        // tx[1] <-> statement_lines[1]
        assert_eq!(txs[1].amount, fmt.transactions[0].statement_lines[1].amount);
        assert_eq!(
            txs[1].operation_type,
            fmt.transactions[0].statement_lines[1].ext_debit_credit_indicator
        );
        assert_eq!(txs[1].date, fmt.transactions[0].statement_lines[1].value_date);
        assert_eq!(txs[1].currency, "EUR");
    }

    fn find_text(camt: &Camt053Format, path: &str) -> Option<String> {
        camt.get_iter()
            .find(|t| t.path().as_str() == path)
            .map(|t| t.text())
    }

    fn find_attr(camt: &Camt053Format, path: &str, attr: &str) -> Option<String> {
        camt.get_iter()
            .find(|t| t.path().as_str() == path)
            .and_then(|t| t.get_attr(attr))
    }

    #[test]
    fn mt940_to_camt053_transfers_core_fields() {
        let mt = MT940Format {
            transactions: vec![get_message()],
            other_data: vec!["".to_string()],
        };
        let camt: Camt053Format = mt.into();

        // 1) AccountId у вас кладётся в IBAN (если выглядит как IBAN)
        assert_eq!(
            find_text(&camt, "/BkToCstmrStmt/Stmt/Acct/Id/IBAN").as_deref(),
            Some("DE12500105170648489890")
        );

        // 2) statement_no и sequence_no:
        // Stmt/Id = "{statement_no}/{sequence_no}"
        assert_eq!(
            find_text(&camt, "/BkToCstmrStmt/Stmt/Id").as_deref(),
            Some("TRN123456")
        );
        // Stmt/ElctrncSeqNb = sequence_no
        assert_eq!(
            find_text(&camt, "/BkToCstmrStmt/Stmt/ElctrncSeqNb").as_deref(),
            Some("00001")
        );

        // 3) Балансы: проверяем, что хотя бы один баланс OPBD/CLBD существует
        // и что Amt имеет Ccy="EUR"
        // (у вас баланс строится как Stmt/Bal/... и код баланса в .../Tp/.../Cd)
        let has_opbd = camt.get_iter().any(|t| {
            t.path().as_str() == "/BkToCstmrStmt/Stmt/Bal/Tp/CdOrPrtry/Cd" && t.text() == "OPBD"
        });
        let has_clbd = camt.get_iter().any(|t| {
            t.path().as_str() == "/BkToCstmrStmt/Stmt/Bal/Tp/CdOrPrtry/Cd" && t.text() == "CLBD"
        });

        assert!(has_opbd, "OPBD (opening balance) must exist in CAMT");
        assert!(has_clbd, "CLBD (closing balance) must exist in CAMT");

        // Amt currency: в вашем коде для Ntry/Amt Ccy берётся из opening_balance.iso_currency_code
        assert_eq!(
            find_attr(&camt, "/BkToCstmrStmt/Stmt/Ntry/Amt", "Ccy").as_deref(),
            Some("EUR")
        );

        // 4) Транзакция: amount и направление
        assert_eq!(
            find_text(&camt, "/BkToCstmrStmt/Stmt/Ntry/Amt").as_deref(),
            Some("1.23")
        );
        // Debit -> "DBIT"
        assert_eq!(
            find_text(&camt, "/BkToCstmrStmt/Stmt/Ntry/CdtDbtInd").as_deref(),
            Some("DBIT")
        );

        // 5) Банк-референс должен оказаться в AcctSvcrRef
        assert_eq!(
            find_text(&camt, "/BkToCstmrStmt/Stmt/Ntry/AcctSvcrRef").as_deref(),
            Some("ABC123")
        );

        // 6) InformationToAccountOwner -> RmtInf/Ustrd
        assert_eq!(
            find_text(&camt, "/BkToCstmrStmt/Stmt/Ntry/NtryDtls/TxDtls/AddtlTxInf").as_deref(),
            Some("TEST PAYMENT")
        );

        // 7) customer_ref + supplementary_details у вас собираются в BkTxCd/Prtry/Cd как "NONREF/SUP"
        assert_eq!(
            find_text(&camt, "/BkToCstmrStmt/Stmt/Ntry/BkTxCd/Prtry/Cd").as_deref(),
            Some("NONREF/SUP")
        );
    }
}
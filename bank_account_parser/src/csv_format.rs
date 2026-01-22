use crate::common::{FormatError, GeneratorFormatError};
use crate::common::debit_credit::DebitOrCredit;
use crate::transactions_holder::{Transaction, TransactionsReader};
use chrono::NaiveDate;
use std::collections::HashMap;

enum State {
    Before,
    Header,
    Data,
    After,
}


#[derive(Default)]
pub struct CSVFormat {
    columns: Vec<String>,
    table: Vec<Vec<String>>,
    other_before: Vec<Vec<String>>,
    other_after: Vec<Vec<String>>,
}

impl GeneratorFormatError for CSVFormat {
    const ERROR_PREFIX: &'static str = "Ошибка разбора таблицы csv";
}

impl CSVFormat {
    fn is_header_like(cells: &[&str]) -> bool {
        for cell in cells {
            if !cell.is_empty() && cell.chars().any(|c| c.is_ascii_digit()) {
                return false;
            }
        }
        true
    }

    fn join_columns(columns: &mut [String], cells: &[&str]) {
        for (index, column) in columns.iter_mut().enumerate() {
            if !cells[index].is_empty() {
                column.push_str(cells[index]);
            }
        }
    }

    /// Разобрать CSV (без встроенных headers) и построить внутреннее представление.
    ///
    /// Формат ожидается «как выгрузка банка»: до таблицы могут быть произвольные строки,
    /// далее идёт заголовок таблицы, начинающийся с колонки `Дата проводки`, затем строки данных,
    /// после пустой строки (полностью пустой ряд) могут идти дополнительные строки.
    ///
    /// # Ошибки
    /// Возвращает [`FormatError`], если:
    /// - не удалось обнаружить заголовок с колонкой `Дата проводки`;
    /// - в результате таблица/колонки получились пустыми;
    /// - входной CSV некорректен на уровне парсера `csv` crate.
    pub fn from_read<R: std::io::Read>(r: &mut R) -> Result<CSVFormat, FormatError> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(r);

        let mut state = State::Before;

        let mut table: Vec<Vec<String>> = Vec::new();
        let mut other_before: Vec<Vec<String>> = Vec::new();
        let mut other_after: Vec<Vec<String>> = Vec::new();
        let mut columns: Vec<String> = Vec::new();
        let mut column_position: (usize, usize) = (0, 0);

        for rec in rdr.records().filter_map(Result::ok) {
            let cells: Vec<&str> = rec.iter().map(|s| s.trim()).collect();

            match state {
                State::Before => {
                    if cells.contains(&"Дата проводки") {
                        let first = cells.iter().position(|s| !s.is_empty());
                        let last = cells.iter().rposition(|s| !s.is_empty());
                        let (Some(f), Some(l)) = (first, last) else {
                            continue;
                        };
                        column_position = (f, l + 1);
                        columns = cells[column_position.0..column_position.1]
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                        state = State::Header;
                    } else {
                        other_before
                            .push(cells.iter().map(|s| s.to_string()).collect::<Vec<String>>());
                    }
                }

                State::Header => {
                    if Self::is_header_like(&cells[column_position.0..column_position.1]) {
                        Self::join_columns(
                            &mut columns,
                            &cells[column_position.0..column_position.1],
                        );
                    } else {
                        state = State::Data;
                        table.push(
                            cells[column_position.0..column_position.1]
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>(),
                        );
                    }
                }

                State::Data => {
                    let all_empty = cells.iter().all(|c| c.is_empty());
                    if all_empty {
                        other_after.push(cells.iter().map(|s| s.to_string()).collect::<Vec<String>>());
                        state = State::After;
                    } else {
                        table.push(
                            cells[column_position.0..column_position.1]
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>(),
                        );
                    }
                }

                State::After => {
                    other_after.push(cells.iter().map(|s| s.to_string()).collect::<Vec<String>>());
                }
            }
        }

        if columns.is_empty() || table.is_empty() {
            return Err(Self::data_format_error(
                "не удалось распарсить данные. В результате разбора список колонок и таблица получились пустыми",
            ));
        }

        Ok(Self {
            columns,
            table,
            other_before,
            other_after,
        })
    }

    /// Записать текущее представление обратно в CSV.
    ///
    /// Запись выполняется в следующем порядке:
    /// 1) строки пролога (`other_before`);
    /// 2) строка заголовка (`columns`);
    /// 3) строки данных (`table`);
    /// 4) строки эпилога (`other_after`).
    ///
    /// # Ошибки
    /// Возвращает [`FormatError`], если запись через `csv::Writer` завершилась ошибкой.
    pub fn write_to<W: std::io::Write>(&mut self, writer: &mut W) -> Result<(), FormatError> {
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_writer(writer);

        for row in &self.other_before {
            if let Err(e) = wtr.write_record(row) {
                return Err(Self::read_write_error(
                    format!("ошибка при дополнительных данных. {}", e).as_str(),
                ));
            }
        }

        if let Err(e) = wtr.write_record(&self.columns) {
            return Err(Self::read_write_error(
                format!("ошибка при записи колонок. {}", e).as_str(),
            ));
        }

        for row in &self.table {
            if let Err(e) = wtr.write_record(row) {
                return Err(Self::read_write_error(
                    format!("ошибка при записи таблицы. {}", e).as_str(),
                ));
            }
        }

        for row in &self.other_after {
            if let Err(e) = wtr.write_record(row) {
                return Err(Self::read_write_error(
                    format!("ошибка при дополнительных данных. {}", e).as_str(),
                ));
            }
        }

        Ok(())
    }
}

impl TransactionsReader for CSVFormat {
    fn collect_transactions(&self) -> Vec<Transaction> {
        let mut transactions = Vec::new();
        let mut index = HashMap::new();
        for cell in self.columns.iter().enumerate() {
            if cell.1 == "Дата проводки" {
                index.insert("Дата проводки".to_string(), cell.0);
            } else if cell.1 == "Сумма по дебету" {
                index.insert("Сумма по дебету".to_string(), cell.0);
            } else if cell.1 == "Сумма по кредиту" {
                index.insert("Сумма по кредиту".to_string(), cell.0);
            }
        }

        for row in &self.table {
            let mut transaction = Transaction::default();

            if let Ok(d) =
                NaiveDate::parse_from_str(row[index["Дата проводки"]].as_str(), "%d.%m.%Y")
            {
                transaction.date = d;
            }

            if row[index["Сумма по дебету"]].is_empty() {
                transaction.operation_type = DebitOrCredit::Credit;
                if let Ok(a) = row[index["Сумма по кредиту"]].replace(",", ".").parse()
                {
                    transaction.amount = a
                }
            } else {
                transaction.operation_type = DebitOrCredit::Debit;
                if let Ok(a) = row[index["Сумма по дебету"]].replace(",", ".").parse()
                {
                    transaction.amount = a
                }
            }
            transactions.push(transaction);
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

    fn minimal_csv() -> String {
        [
            "Какая-то строка до таблицы",
            "Ещё строка",
            "Дата проводки,Сумма по дебету,Сумма по кредиту",
            "2026-01-20,123.45,",
            "2026-01-21,,10.00",
            ",,,",
            "Строка после таблицы",
        ].join("\n")
    }

    #[test]
    fn parse_includes_last_column() {
        let data = minimal_csv();
        let mut cur = Cursor::new(data.as_bytes());
        let fmt = CSVFormat::from_read(&mut cur).expect("parse");

        assert_eq!(
            fmt.columns,
            vec![
                "Дата проводки".to_string(),
                "Сумма по дебету".to_string(),
                "Сумма по кредиту".to_string()
            ]
        );
        assert_eq!(fmt.table.len(), 2);
        assert_eq!(fmt.other_before.len(), 2);
        assert_eq!(fmt.other_after.len(), 2);
    }

    #[test]
    fn collect_transactions_parses_debit_credit_and_amount() {
        let data = minimal_csv();
        let mut cur = Cursor::new(data.as_bytes());
        let fmt = CSVFormat::from_read(&mut cur).expect("parse");

        let txs = fmt.collect_transactions();
        assert_eq!(txs.len(), 2);

        assert_eq!(txs[0].date, NaiveDate::from_ymd_opt(2026, 1, 20).unwrap());
        assert_eq!(txs[0].operation_type, DebitOrCredit::Debit);
        assert_ne!(txs[0].amount.to_string(), Transaction::default().amount.to_string());

        assert_eq!(txs[1].date, NaiveDate::from_ymd_opt(2026, 1, 21).unwrap());
        assert_eq!(txs[1].operation_type, DebitOrCredit::Credit);
        assert_ne!(txs[1].amount.to_string(), Transaction::default().amount.to_string());
    }

    #[test]
    fn write_roundtrip_keeps_structure() {
        let data = minimal_csv();
        let mut cur = Cursor::new(data.as_bytes());
        let mut fmt = CSVFormat::from_read(&mut cur).expect("parse");

        let mut out: Vec<u8> = Vec::new();
        fmt.write_to(&mut out).expect("write");

        let written = String::from_utf8(out).expect("utf8");
        assert!(written.contains("Дата проводки"));
        assert!(written.contains("2026-01-20"));
        assert!(written.contains("2026-01-21"));

        let mut cur2 = Cursor::new(written.as_bytes());
        let fmt2 = CSVFormat::from_read(&mut cur2).expect("parse2");
        assert_eq!(fmt2.columns, fmt.columns);
        assert_eq!(fmt2.table.len(), fmt.table.len());
    }

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn csv_collect_transactions_converts_rows_to_transactions() {
        // Исходные данные CSV
        let data = minimal_csv();
        let mut cur = Cursor::new(data.as_bytes());
        let fmt = CSVFormat::from_read(&mut cur).expect("parse");

        // Конвертация в Transaction
        let txs = fmt.collect_transactions();

        assert_eq!(txs.len(), 2);

        // row 0 -> tx 0 (Debit)
        assert_eq!(txs[0].date, NaiveDate::from_ymd_opt(2026, 1, 20).unwrap());
        assert_eq!(txs[0].operation_type, DebitOrCredit::Debit);
        assert_eq!(txs[0].amount, dec("123.45")); // ',' -> '.'

        // row 1 -> tx 1 (Credit)
        assert_eq!(txs[1].date, NaiveDate::from_ymd_opt(2026, 1, 21).unwrap());
        assert_eq!(txs[1].operation_type, DebitOrCredit::Credit);
        assert_eq!(txs[1].amount, dec("10.00"));
    }

}
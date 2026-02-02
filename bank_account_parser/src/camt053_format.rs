use crate::camt053_iterator::Camt053Iter;
use crate::common::{FormatError, GeneratorFormatError};
use crate::common::debit_credit::DebitOrCredit;
use crate::mt940_format::{AvailableBalance, MT940Format};
use crate::transactions_holder::{Transaction, TransactionsReader};
use chrono::NaiveDate;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;
use std::cell::RefCell;
use std::io::{BufReader, Write};
use std::rc::{Rc, Weak};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Tag {
    pub name: String,
    pub text: Option<String>,
    pub attrs: Vec<(String, String)>,
    pub childrens: Vec<Rc<RefCell<Tag>>>,
    pub parent: Weak<RefCell<Tag>>,
}

#[derive(Default)]
pub struct Camt053Format {
    root: Rc<RefCell<Tag>>,
}

impl GeneratorFormatError for Camt053Format {
    const ERROR_PREFIX: &'static str = "Ошибка разбора формата camt053";
}

impl Camt053Format {
    fn looks_like_iban(s: &str) -> bool {
        let x: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        let x = x.as_str();

        if !(15..=34).contains(&x.len()) {
            return false;
        }
        true
    }

    /// Разобрать CAMT.053 (ISO 20022) из потока ввода и построить дерево XML-тегов.
    ///
    /// Метод читает XML, строит внутреннее дерево [`Tag`] с родителями/детьми и возвращает [`Camt053Format`].
    /// Текстовые узлы нормализуются с `trim_text(true)`, то есть пробельные узлы отбрасываются.
    ///
    /// # Ошибки
    /// Возвращает [`FormatError`], если XML некорректен, содержит неподдерживаемые/битые атрибуты
    /// или произошла ошибка чтения.
    ///
    pub fn from_read<R: std::io::Read>(r: &mut R) -> Result<Camt053Format, FormatError> {
        let buf_reader = BufReader::new(r);
        let mut reader = Reader::from_reader(buf_reader);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        // Создаем ВИРТУАЛЬНЫЙ корневой тег, который будет хранить всех детей
        let virtual_root = Rc::new(RefCell::new(Tag::default()));

        let mut previous_tag = Rc::clone(&virtual_root); // начинаем с виртуального корня

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let new_tag = Rc::new(RefCell::new(Tag {
                        name: String::from_utf8_lossy(e.name().as_ref()).to_string(),
                        text: None,
                        attrs: Vec::new(),
                        childrens: Vec::new(),
                        parent: Rc::downgrade(&previous_tag),
                    }));

                    {
                        let mut new_tag_mut = new_tag.borrow_mut();
                        for attr_result in e.attributes() {
                            let attr = match attr_result {
                                Ok(a) => a,
                                Err(_) => {
                                    return Err(Self::unknown_value_error(
                                        format!(
                                            "не удалось распарсить атрибуты тега {}",
                                            new_tag_mut.name
                                        )
                                        .as_str(),
                                    ));
                                }
                            };
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let value = String::from_utf8_lossy(&attr.value).to_string();
                            new_tag_mut.attrs.push((key, value));
                        }
                    }
                    previous_tag
                        .borrow_mut()
                        .childrens
                        .push(Rc::clone(&new_tag));
                    previous_tag = new_tag;
                }
                Ok(Event::Text(e)) => {
                    let text = String::from_utf8_lossy(e.as_ref()).to_string();
                    let mut tag_mut = previous_tag.borrow_mut();
                    if tag_mut.name.is_empty() {
                        Err(Self::data_format_error(
                            format!("не найден тег которому принадлежит текст {text}").as_str(),
                        ))?
                    };
                    tag_mut.text = Some(text);
                }
                Ok(Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    previous_tag = {
                        let tag = previous_tag.borrow();
                        if tag.name.is_empty() || tag.name != name {
                            Err(Self::data_format_error(
                                format!(
                                    "найден закрывающийся тег {name} но ожидался тег {}",
                                    tag.name.as_str()
                                )
                                .as_str(),
                            ))?;
                        }
                        let Some(result) = tag.parent.upgrade() else {
                            Err(Self::unknown_error("ошибка чтения данных"))?
                        };
                        result
                    }
                }
                Ok(Event::Comment(_)) => continue,
                Ok(Event::Eof) => break,
                Err(e) => Err(Self::read_write_error(
                    format!("не удалось разложить xml файл на теги {}", e).as_str(),
                ))?,
                _ => {
                    Err(Self::read_write_error(r#"при разборе файла встретился неизвестный раздел.
                    Сейчас поддерживаются блоки <...>, </...>, комментарии,
                    текст относящийся к тегам и символы завершения файла."#))?
                }
            }
            buf.clear();
        }

        let root = {
            let tag = previous_tag.borrow();
            if tag.childrens.is_empty() {
                Err(Self::data_format_error(
                    "не удалось прочитать ни одного тега",
                ))?
            };
            tag.childrens[0].clone()
        };
        Ok(Self { root })
    }

    fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        tag: &Rc<RefCell<Tag>>,
    ) -> Result<(), FormatError> {
        let tag = tag.borrow();
        let mut root = BytesStart::new(tag.name.clone());
        for attr in &tag.attrs {
            let (key, value) = attr; // Разбираем кортеж
            root.push_attribute((key.as_str(), value.as_str()));
        }
        writer.write_event(Event::Start(root))?;
        if let Some(ref text) = tag.text {
            writer.write_event(Event::Text(BytesText::new(text)))?;
        }
        for child in &tag.childrens {
            self.write(writer, child)?;
        }
        writer.write_event(Event::End(BytesEnd::new(tag.name.clone())))?;
        Ok(())
    }

    /// Записать текущее дерево CAMT.053 обратно в XML.
    ///
    /// Если корневой узел является “виртуальным” (пустое имя тега, как после [`from_read`]),
    /// то в выход будет записано **его содержимое** (дети), а не пустой контейнер.
    ///
    /// # Ошибки
    /// Возвращает [`FormatError`] при ошибке записи в `writer` или при ошибке сериализации XML.
    ///
    pub fn write_to<W: Write>(&mut self, writer: &mut W) -> Result<(), FormatError> {
        self.write(&mut Writer::new(writer), &self.root)
    }

    /// Получить итератор (обход в глубину, pre-order) по всем тегам документа.
    ///
    /// Итератор возвращает [`crate::camt053_iterator::TagView`], содержащий:
    /// - `path()` — путь вида `A/B/C`
    /// - `text()` — текстовое содержимое (пустая строка, если отсутствует)
    ///
    pub fn get_iter(&self) -> Camt053Iter {
        Camt053Iter::new(self.root.clone())
    }

    fn set_parent(start: &Rc<RefCell<Tag>>) {
        for child in start.borrow_mut().childrens.iter() {
            child.borrow_mut().parent = Rc::downgrade(start);
            Self::set_parent(child);
        }
    }

}

impl From<MT940Format> for Camt053Format {
    fn from(v: MT940Format) -> Self {
        let crt_with_text = |name: &str, text: Option<String>| {
            Rc::new(RefCell::new(Tag {
                name: name.to_string(),
                text,
                attrs: Vec::new(),
                childrens: Vec::new(),
                parent: Weak::new(),
            }))
        };

        let crt_with_child = |name: &str, child: &[Rc<RefCell<Tag>>]| {
            Rc::new(RefCell::new(Tag {
                name: name.to_string(),
                text: None,
                attrs: Vec::new(),
                childrens: child.to_vec(),
                parent: Weak::new(),
            }))
        };

        let debi_cred_name = |x: &DebitOrCredit| -> &str {
            match x {
                DebitOrCredit::Debit | DebitOrCredit::ReverseDebit => "DBIT",
                DebitOrCredit::Credit | DebitOrCredit::ReverseCredit => "CRDT",
            }
        };

        let root = crt_with_child(
            "BkToCstmrStmt",
            [crt_with_child(
                "GrpHdr",
                [crt_with_text("MsgId", Some(Uuid::new_v4().to_string()))].as_ref(),
            )].as_ref(),
        );

        let root_ref = Rc::clone(&root);

        for transaction in v.into_iter() {
            let stmt = crt_with_text("Stmt", None);
            let mut stmt_child: Vec<Rc<RefCell<Tag>>>;

            {
                // <Id>
                let mut id_text = transaction.statement_no.clone();
                transaction
                    .sequence_no
                    .as_deref()
                    .map(|seq| format!("/{}", seq))
                    .inspect(|s| id_text.push_str(s));
                let id = crt_with_text("Id", Some(id_text));

                stmt_child = vec![id];
            }

            if let Some(seq) = &transaction.sequence_no {
                let t = crt_with_text("ElctrncSeqNb", Some(seq.clone()));
                stmt_child.push(t);
            }

            {
                // <FrToDt>
                stmt_child.push(crt_with_child(
                    "FrToDt",
                    [
                        crt_with_text(
                            "FrDtTm",
                            Some(transaction.opening_balance.date.format("%Y-%m-%dT00:00:00").to_string()),
                        ),
                        crt_with_text(
                            "ToDtTm",
                            Some(transaction.closing_balance.date.format("%Y-%m-%dT00:00:00").to_string()),
                        ),
                    ].as_ref(),
                ));
            }

            {
                // <Acct>
                let acct = crt_with_text("Acct", None);
                if Self::looks_like_iban(&transaction.account_id) {
                    acct.borrow_mut().childrens.push(crt_with_child(
                        "Id",
                        [crt_with_text("IBAN", Some(transaction.account_id.clone()))].as_ref(),
                    ));
                } else {
                    acct.borrow_mut().childrens.push(crt_with_child(
                        "Id",
                        [crt_with_child(
                            "Othr",
                            [crt_with_text("Id", Some(transaction.account_id.clone()))].as_ref(),
                        )].as_ref(),
                    ))
                }

                acct.borrow_mut().childrens.push(crt_with_text(
                    "Ccy",
                    Some(transaction.opening_balance.iso_currency_code.clone()),
                ));
                stmt_child.push(acct)
            }

            {
                let balance2tag = |bal: &AvailableBalance, _type: &str| {
                    let amt = crt_with_text("Amt", Some(bal.amount.to_string()));
                    amt.borrow_mut()
                        .attrs
                        .push(("Ccy".to_string(), bal.iso_currency_code.clone()));

                    crt_with_child(
                        "Bal",
                        [
                            amt,
                            crt_with_text("CdtDbtInd", Some(debi_cred_name(&bal.debit_credit_indicator).to_string())),
                            crt_with_child(
                                "Dt",
                                [crt_with_text(
                                    "Dt",
                                    Some(bal.date.format("%Y-%m-%dT00:00:00").to_string()),
                                )].as_ref(),
                            ),
                            crt_with_child(
                                "Tp",
                                [crt_with_child(
                                    "CdOrPrtry",
                                    [crt_with_text("Cd", Some(_type.to_string()))].as_ref(),
                                )].as_ref(),
                            ),
                        ].as_ref(),
                    )
                };

                // <Bal>
                let op: AvailableBalance = transaction.opening_balance.clone().into();
                stmt_child.push(balance2tag(&op, "OPBD"));
                let cb: AvailableBalance = transaction.closing_balance.clone().into();
                stmt_child.push(balance2tag(&cb, "CLBD"));
                if let Some(bal) = &transaction.closing_available_balance {
                    stmt_child.push(balance2tag(&bal, "CLAV"));
                }
                if let Some(bal) = &transaction.forward_available_balance {
                    stmt_child.push(balance2tag(&bal, "FWAV"));
                }
            }

            {
                // <TxsSummry>
                let mut cbt = 0;
                let mut cbt_sum: Decimal = Decimal::zero();
                let mut dbt = 0;
                let mut dbt_sum: Decimal = Decimal::zero();
                for x in &transaction.statement_lines {
                    if x.ext_debit_credit_indicator == DebitOrCredit::Credit
                        || x.ext_debit_credit_indicator == DebitOrCredit::ReverseCredit
                    {
                        cbt += 1;
                        cbt_sum += x.amount;
                    } else if x.ext_debit_credit_indicator == DebitOrCredit::Debit
                        || x.ext_debit_credit_indicator == DebitOrCredit::ReverseDebit
                    {
                        dbt += 1;
                        dbt_sum += x.amount;
                    }
                }

                stmt_child.push(crt_with_child(
                    "TxsSummry",
                    [
                        crt_with_child(
                            "TtlCdtNtries",
                            [
                                crt_with_text("NbOfNtries", Some(cbt.to_string())),
                                crt_with_text("Sum", Some(cbt_sum.to_string())),
                            ].as_ref(),
                        ),
                        crt_with_child(
                            "TtlDbtNtries",
                            [
                                crt_with_text("NbOfNtries", Some(dbt.to_string())),
                                crt_with_text("Sum", Some(dbt_sum.to_string())),
                            ].as_ref(),
                        ),
                        crt_with_child(
                            "TtlNtries",
                            [
                                crt_with_text(
                                    "NbOfNtries",
                                    Some(transaction.statement_lines.len().to_string()),
                                ),
                                crt_with_text(
                                    "TtlNetNtryAmt",
                                    Some((cbt_sum - dbt_sum).to_string()),
                                ),
                                crt_with_text(
                                    "CdtDbtInd",
                                    Some((if (cbt_sum - dbt_sum) > 0.into() {
                                        "CRDT"
                                    } else {
                                        "DBIT"
                                    }).to_string())
                                ),
                            ].as_ref())
                    ].as_ref(),
                ))
            }

            {
                // <Ntry>
                for stat in &transaction.statement_lines {
                    let amt = crt_with_text("Amt", Some(stat.amount.to_string()));
                    amt.borrow_mut().attrs.push((
                        "Ccy".to_string(),
                        transaction.opening_balance.iso_currency_code.clone(),
                    ));
                    let mut cd_text = stat.customer_ref.clone();
                    if let Some(sup_det) = &stat.supplementary_details {
                        cd_text += "/";
                        cd_text += sup_det.as_str();
                    };

                    let mut val = "false";
                    if stat.ext_debit_credit_indicator == DebitOrCredit::ReverseCredit
                        || stat.ext_debit_credit_indicator == DebitOrCredit::ReverseDebit
                    {
                        val = "true";
                    }

                    let ntry = crt_with_child(
                        "Ntry",
                        [
                            amt,
                            crt_with_text(
                                "CdtDbtInd",
                                Some(debi_cred_name(&stat.ext_debit_credit_indicator).to_string()),
                            ),
                            crt_with_text("RvslInd", Some(val.to_string())),
                            crt_with_child(
                                "ValDt",
                                [crt_with_text(
                                    "Dt",
                                    Some(stat.value_date.format("%Y-%m-%dT00:00:00").to_string()),
                                )].as_ref(),
                            ),
                            crt_with_text("Sts", Some("BOOK".to_string())),
                            crt_with_child(
                                "BkTxCd",
                                [crt_with_child(
                                    "Prtry",
                                    [
                                        crt_with_text("Cd", Some(cd_text.to_string())),
                                        crt_with_text("Issr", Some("MT940".to_string())),
                                    ].as_ref(),
                                )].as_ref(),
                            ),
                        ].as_ref(),
                    );
                    if let Some(entry) = stat.entry_date {
                        ntry.borrow_mut().childrens.push(crt_with_child(
                            "BookgDt",
                            [crt_with_text(
                                "Dt",
                                Some(entry.format("%Y-%m-%dT00:00:00").to_string()),
                            )].as_ref(),
                        ))
                    }

                    if let Some(s) = &stat.supplementary_details {
                        ntry.borrow_mut().childrens.push(crt_with_text("AddtlTxInf", Some(s.clone())))
                    }

                    let refs = crt_with_child(
                        "Refs",
                        [crt_with_text("EndToEndId", Some(stat.customer_ref.clone()))].as_ref(),
                        );
                    if let Some(bank) = &stat.bank_ref {
                        ntry.borrow_mut()
                            .childrens
                            .push(crt_with_text("AcctSvcrRef", Some(bank.clone())));
                        refs.borrow_mut()
                            .childrens
                            .push(crt_with_text("TxId", Some(bank.clone())));
                    }

                    ntry.borrow_mut().childrens.push(crt_with_child(
                        "NtryDtls",
                        [crt_with_child(
                            "TxDtls",
                            [
                                refs,
                                crt_with_text("AddtlTxInf", stat.information_to_account_owner.clone()),
                            ].as_ref(),
                        )].as_ref()
                    ));

                    stmt_child.push(ntry);
                }
            }

            stmt.borrow_mut().childrens = stmt_child;
            root_ref.borrow_mut().childrens.push(stmt)
        }
        Self::set_parent(&root_ref);

        Self { root: root_ref }
    }
}

impl TransactionsReader for Camt053Format {
    fn collect_transactions(&self) -> Vec<Transaction> {
        let mut transactions = Vec::new();
        let mut transaction = None;
        for tag in self.get_iter() {
            let path = tag.path();
            let Some(s) = path.find("/Stmt") else { continue };
            match &path[s..] {
                "/Stmt/Ntry" => {
                    if let Some(t) = transaction {
                        transactions.push(t);
                    }
                    transaction = Some(Transaction::default());
                }
                "/Stmt/Ntry/Amt" => {
                    if let Some(t) = &mut transaction {
                        if let Ok(amount) = tag.text().replace(",", ".").parse() {
                            t.amount = amount;
                        }
                        if let Some(curr) = tag.get_attr("Ccy") {
                            t.currency = curr;
                        }
                    }
                }
                "/Stmt/Ntry/CdtDbtInd" => {
                    if let Some(t) = &mut transaction {
                        match tag.text().as_str() {
                            "DBIT" => t.operation_type = DebitOrCredit::Debit,
                            "CRDT" => t.operation_type = DebitOrCredit::Credit,
                            _ => t.operation_type = DebitOrCredit::Debit,
                        }
                    }
                }
                "/Stmt/Ntry/ValDt/Dt" => {
                    let val = tag.text();
                    if let Some(t) = &mut transaction
                        && let Ok(d) = NaiveDate::parse_from_str(&val[..10], "%Y-%m-%d")
                    {
                        t.date = d;
                    }
                }
                _ => (),
            }
        }
        if let Some(t) = transaction {
        transactions.push(t);
        }
        transactions
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn from_read_parses_basic_xml_and_iter_yields_paths() {
        let xml = r#"<Document><Stmt><Id>STATEMENT-1</Id><Ntry><Amt Ccy="EUR">12.34</Amt><CdtDbtInd>CRDT</CdtDbtInd><ValDt><Dt>2026-01-01</Dt></ValDt></Ntry></Stmt></Document>"#;
        let mut cur = Cursor::new(xml);
        let camt = Camt053Format::from_read(&mut cur).unwrap();

        let paths: Vec<String> = camt.get_iter().map(|v| v.path().to_string()).collect();

        assert!(paths.iter().any(|p| p.ends_with("Document")));
        assert!(paths.iter().any(|p| p.ends_with("Stmt/Id")));
        assert!(paths.iter().any(|p| p.ends_with("Stmt/Ntry/Amt")));
    }

    #[test]
    fn write_to_does_not_serialize_virtual_root() {
        let xml = r#"<Document><Stmt><Id>1</Id></Stmt></Document>"#;
        let mut cur = Cursor::new(xml);
        let mut camt = Camt053Format::from_read(&mut cur).unwrap();

        let mut out = Vec::new();
        camt.write_to(&mut out).unwrap();
        let s = String::from_utf8(out).unwrap();

        assert!(!s.contains("<>"));
        assert!(s.contains("<Document>"));
        assert!(s.contains("</Document>"));
    }

    #[cfg(test)]
    mod camt_to_mt_tests {
        use crate::camt053_format::Camt053Format;
        use crate::mt940_format::MT940Format;
        use crate::common::debit_credit::DebitOrCredit;
        use chrono::NaiveDate;
        use rust_decimal::Decimal;
        use std::io::Cursor;
        use std::str::FromStr;

        fn d(y: i32, m: u32, day: u32) -> NaiveDate {
            NaiveDate::from_ymd_opt(y, m, day).unwrap()
        }

        fn dec(s: &str) -> Decimal {
            Decimal::from_str(s).unwrap()
        }

        #[test]
        fn camt053_to_mt940_transfers_message_level_fields_and_balances() {
            let xml = r#"
                <BkToCstmrStmt>
                    <GrpHdr>
                        <OrgnlBizQry>
                            <MsgId>BASE-REF</MsgId>
                        </OrgnlBizQry>
                    </GrpHdr>

                    <Stmt>
                        <Id>TRN-1</Id>
                        <Acct><Id><IBAN>DE12500105170648489890</IBAN></Id></Acct>
                        <ElctrncSeqNb>00001</ElctrncSeqNb>
                        <LglSeqNb>001</LglSeqNb>
                        <AddtlStmtInf>STATEMENT INFO</AddtlStmtInf>

                        <Bal>
                            <Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp>
                            <Amt Ccy="EUR">100.00</Amt>
                            <CdtDbtInd>CRDT</CdtDbtInd>
                            <Dt><Dt>2024-01-01</Dt></Dt>
                        </Bal>

                        <Bal>
                            <Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp>
                            <Amt Ccy="EUR">98.77</Amt>
                            <CdtDbtInd>CRDT</CdtDbtInd>
                            <Dt><Dt>2024-01-02</Dt></Dt>
                        </Bal>
                    </Stmt>
                </BkToCstmrStmt>
                "#;

            let mut cur = Cursor::new(xml);
            let camt = Camt053Format::from_read(&mut cur).unwrap();

            let mt: MT940Format = camt.into();

            // Ожидаем, что хотя бы одно сообщение появилось
            assert!(!mt.transactions.is_empty());

            let msg = &mt.transactions[0];

            // Message-level поля
            assert_eq!(msg.transaction_ref_no, "TRN-1");
            assert_eq!(msg.account_id, "DE12500105170648489890");
            assert_eq!(msg.statement_no, "00001");
            assert_eq!(msg.sequence_no.as_deref(), Some("001"));
            assert_eq!(msg.information_to_account_owner.as_deref(), Some("STATEMENT INFO"));

            // Opening balance
            assert_eq!(msg.opening_balance.iso_currency_code, "EUR");
            assert_eq!(msg.opening_balance.debit_credit_indicator, DebitOrCredit::Credit);
            assert_eq!(msg.opening_balance.date, d(2024, 1, 1));
            assert_eq!(msg.opening_balance.amount, dec("100.00"));

            // Closing balance
            assert_eq!(msg.closing_balance.iso_currency_code, "EUR");
            assert_eq!(msg.closing_balance.debit_credit_indicator, DebitOrCredit::Credit);
            assert_eq!(msg.closing_balance.date, d(2024, 1, 2));
            assert_eq!(msg.closing_balance.amount, dec("98.77"));
        }
    }
}
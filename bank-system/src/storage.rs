use std::collections::HashMap;
use std::fs::File;
use std::{fs, io};
use std::io::{BufRead, Cursor, Read, Write};
use std::path::Path;
use crate::{Balance, Name, Storage};

impl Storage {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn add_user(&mut self, name: Name) -> Option<Balance> {
        match self.accounts.entry(name) {
            std::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(0);
                Some(0)
            }
            std::collections::hash_map::Entry::Occupied(_) => None,
        }
    }

    pub fn remove_user(&mut self, name: &Name) -> Option<Balance> {
        self.accounts.remove(name)
    }

    pub fn get_balance(&self, name: &Name) -> Option<Balance> {
        self.accounts.get(name).copied()
    }

    pub fn deposit(&mut self, name: &Name, amount: Balance) -> Result<(), String> {
        if let Some(balance) = self.accounts.get_mut(name) {
            *balance += amount;
            Ok(())
        } else {
            Err("Пользователь не найден".into())
        }
    }

    pub fn withdraw(&mut self, name: &Name, amount: Balance) -> Result<(), String> {
        if let Some(balance) = self.accounts.get_mut(name) {
            if *balance >= amount {
                *balance -= amount;
                Ok(())
            } else {
                Err("Недостаточно средств".into())
            }
        } else {
            Err("Пользователь не найден".into())
        }
    }

    pub fn load_data(file: &str) -> Storage {
        let mut storage = Storage::new();
        if Path::new(file).exists()
        {
            let file = File::open(file).unwrap();
            let reader = io::BufReader::new(file);
            for line in reader.lines().map_while(Result::ok) {
                let parts: Vec<&str> = line.trim().split(',').collect();
                if parts.len() == 2 {
                    let name = parts[0].to_string();
                    let balance: i64 = parts[1].parse().unwrap_or(0);
                    storage.add_user(name.clone());
                    let _ = storage.deposit(&name, balance);
                }
            }
        } else {
            for u in ["John", "Alice", "Bob", "Vasya"] {
                storage.add_user(u.to_string());
            }
        }
        storage
    }
    pub fn save(&self, file: &str) {
        let mut data = String::new();
        for (name, balance) in self.get_all() {
            data.push_str(&format!("{name},{balance}\n"));
        }
        fs::write(file, data).expect("Не удалось записать файл");
    }

    pub fn get_all(&self) -> Vec<(Name, Balance)> {
        self.accounts.iter().map(|(a, b)| (a.clone(), *b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, BufWriter, Write};
    use super::*;

    #[test]
    fn test_new_storage_is_empty() {
        let bank = Storage::new();
        assert_eq!(bank.accounts.len(), 0);
    }

    #[test]
    fn test_add_user() {
        let mut storage = Storage::new();
        assert_eq!(storage.add_user("Alice".to_string()), Some(0)); // новый пользователь
        assert_eq!(storage.add_user("Alice".to_string()), None); // уже существует
    }

    #[test]
    fn test_remove_user() {
        let mut storage = Storage::new();
        storage.add_user("Bob".to_string());
        storage.deposit(&"Bob".to_string(), 100).unwrap();

        assert_eq!(storage.remove_user(&"Bob".to_string()), Some(100)); // удаляем и получаем баланс
        assert_eq!(storage.remove_user(&"Bob".to_string()), None); // второй раз — не найден
    }

    #[test]
    fn test_deposit_and_withdraw() {
        let mut storage = Storage::new();
        storage.add_user("Charlie".to_string());

        // Пополнение
        assert!(storage.deposit(&"Charlie".to_string(), 200).is_ok());
        assert_eq!(storage.get_balance(&"Charlie".to_string()), Some(200));

        // Успешное снятие
        assert!(storage.withdraw(&"Charlie".to_string(), 150).is_ok());
        assert_eq!(storage.get_balance(&"Charlie".to_string()), Some(50));

        // Ошибка: недостаточно средств
        assert!(storage.withdraw(&"Charlie".to_string(), 100).is_err());
        assert_eq!(storage.get_balance(&"Charlie".to_string()), Some(50));
    }

    #[test]
    fn test_nonexistent_user() {
        let mut storage = Storage::new();

        // Депозит несуществующему пользователю
        assert!(storage.deposit(&"Dana".to_string(), 100).is_err());

        // Снятие у несуществующего пользователя
        assert!(storage.withdraw(&"Dana".to_string(), 50).is_err());

        // Баланс у несуществующего пользователя
        assert_eq!(storage.get_balance(&"Dana".to_string()), None);
    }

    #[test]
    fn test_load_data_existing_cursor() {
        let data = b"John,100\nAlice,200\nBob,50\n";
        let mut cursor = Cursor::new(&data[..]);

        let mut storage = Storage::new();
        let reader = BufReader::new(&mut cursor);
        for line in reader.lines() {
            let line = line.unwrap();
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 2 {
                let name = parts[0].to_string();
                let balance = parts[1].parse::<i64>().unwrap_or(0);
                storage.add_user(name.clone());
                storage.deposit(&name, balance).unwrap();
            }
        }
        assert_eq!(storage.get_balance(&"John".to_string()), Some(100));
        assert_eq!(storage.get_balance(&"Alice".to_string()), Some(200));
        assert_eq!(storage.get_balance(&"Bob".to_string()), Some(50));
        assert_eq!(storage.get_balance(&"Vasya".to_string()), None);
    }

    #[test]
    fn test_save_creates_cursor_with_correct_data() {
        // Создаём Storage и добавляем пользователей
        let mut storage = Storage::new();
        storage.add_user("John".to_string());
        storage.add_user("Alice".to_string());
        storage.deposit(&"John".to_string(), 150).unwrap();
        storage.deposit(&"Alice".to_string(), 300).unwrap();

        let buffer = Vec::new();
        let mut cursor = Cursor::new(buffer);
        {
            let mut writer = BufWriter::new(&mut cursor);
            for (name, balance) in storage.get_all() {
                writeln!(writer, "{name},{balance}").unwrap();
            }
            writer.flush().unwrap();
        }

        cursor.set_position(0);
        let lines: Vec<String> = BufReader::new(cursor).lines().map(|l| l.unwrap()).collect();
        assert_eq!(lines, vec!["Alice,300", "John,150"]);

    }
}
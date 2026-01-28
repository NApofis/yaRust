use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use rand::Rng;
use std::time::Duration;

use crate::vault::{Item, Vault, VaultError};

pub fn handle_client(stream: TcpStream, vault: Arc<Mutex<Vault>>) {
    // клонируем stream: один экземпляр для чтения (обёрнут в BufReader), другой — для записи
    let mut writer = stream.try_clone().expect("failed to clone stream");
    let mut reader = BufReader::new(stream);

    // send initial prompt
    let _ = writer.write_all(b"Welcome to the Vault!\n");
    let _ = writer.flush();

    let mut line = String::new();
    loop {
        line.clear();
        // read_line ждёт '\n' — nc отправляет строку по нажатию Enter
        match reader.read_line(&mut line) {
            Ok(0) => {
                // EOF — клиент закрыл соединение
                return;
            }
            Ok(_) => {
                let input = line.trim();
                if input.is_empty() {
                    let _ = writer.flush();
                    continue;
                }

                let mut parts = input.split_whitespace();
                let response = match parts.next() {
                    Some("PUT") => {
                        let id = parts.next().and_then(|s| s.parse::<u32>().ok());
                        let name = parts.next();
                        let size = parts.next().and_then(|s| s.parse::<u32>().ok());

                        if let (Some(id), Some(name), Some(size)) = (id, name, size) {
                            let item = Item {
                                name: name.to_string(),
                                size,
                            };
                            let mut v = vault.lock().unwrap();
                            match v.put(id, item, 100) {
                                Ok(_) => "OK: item stored\n".to_string(),
                                Err(VaultError::VaultFull) => "ERROR: vault full\n".to_string(),
                                Err(VaultError::CellFull) => "ERROR: cell full\n".to_string(),
                                _ => "ERROR: unknown\n".to_string(),
                            }
                        } else {
                            "ERROR: usage PUT <id> <name> <size>\n".to_string()
                        }
                    }

                    Some("GET") => {
                        if let Some(id_str) = parts.next() {
                            if let Ok(id) = id_str.parse::<u32>() {
                                let v = vault.lock().unwrap();
                                match v.get(id) {
                                    Ok(Some(items)) => items,
                                    Ok(None) => "Cell is empty\n".to_string(),
                                    Err(VaultError::CellNotFound) => {
                                        "ERROR: cell not found\n".to_string()
                                    }
                                    _ => "ERROR: unknown\n".to_string(),
                                }
                            } else {
                                "ERROR: invalid id\n".to_string()
                            }
                        } else {
                            "ERROR: usage GET <id>\n".to_string()
                        }
                    }

                    Some("LIST") => {
                        let v = vault.lock().unwrap();
                        v.list().unwrap_or_else(|| "Vault is empty\n".to_string())
                    }

                    Some("EXIT") => {
                        let _ = writer.write_all(b"BYE\n");
                        let _ = writer.flush();
                        return;
                    }

                    Some("TAKE") => {
                        let id = parts.next().and_then(|s| s.parse::<u32>().ok());
                        let name = parts.next();

                        if let (Some(id), Some(name)) = (id, name) {
                            let mut v = vault.lock().unwrap();
                            match v.take(id, name) {
                                Ok(item) => {
                                    format!("OK: item taken {} {}\n", item.name, item.size)
                                }
                                Err(VaultError::CellNotFound) => {
                                    "ERROR: cell not found\n".to_string()
                                }
                                Err(VaultError::ItemNotFound) => {
                                    "ERROR: item not found\n".to_string()
                                }
                                Err(_) => "ERROR: unknown\n".to_string(),
                            }
                        } else {
                            "ERROR: usage TAKE <id> <name>\n".to_string()
                        }
                    }

                    Some("PING") => {
                        let mut rng = rand::rng();

                        // Случайная задержка от 1 до 5 секунд
                        let delay_secs = rng.random_range(1..=5);
                        std::thread::sleep(Duration::from_secs(delay_secs));
                        "PONG\n".to_string()
                    }

                    _ => "ERROR: unknown command\n".to_string(),
                };

                // отправляем ответ и снова показываем prompt
                let _ = writer.write_all(response.as_bytes());
                let _ = writer.flush();
            }
            Err(_) => {
                // ошибка чтения — закрываем
                return;
            }
        }
    }
}
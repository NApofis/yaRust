use std::any::Any;
use std::sync::Mutex;

pub trait Logger {
    fn log(&self, message: &str);

    // важно: возвращаем &dyn Any, а не Self
    // и не делаем метод generic — тогда он будет object safe
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone)]
pub struct ConsoleLogger;

impl Logger for ConsoleLogger {
    fn log(&self, message: &str) {
        println!("[console] {message}");
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct MemoryLogger {
    entries: Mutex<Vec<String>>,
}

impl MemoryLogger {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn get_entries(&self) -> Vec<String> {
        self.entries.lock().unwrap().clone()
    }
}

impl Logger for MemoryLogger {
    fn log(&self, message: &str) {
        self.entries.lock().unwrap().push(message.to_string());
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

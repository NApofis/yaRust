use room_monitoring::{MetricsSender, RoomMetrics};
use std::env;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let target_addr = args.get(1).map(|s| s.as_str()).unwrap_or("127.0.0.1:8080");
    let interval_ms = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);

    println!("🚀 Запуск имитатора датчиков банковского хранилища");
    println!("📍 Целевой адрес: {}", target_addr);
    println!("⏱️  Интервал отправки: {} мс", interval_ms);
    println!("──────────────────────────────────────────────────");

    let sender = MetricsSender::new("127.0.0.1:0")?;

    // Бесконечный цикл отправки
    sender.start_broadcasting(target_addr.to_string(), interval_ms)?;

    Ok(())
}
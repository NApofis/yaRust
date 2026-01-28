// examples/demo.rs

use room_monitoring::{MetricsReceiver, MetricsSender, RoomMetrics};
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Демонстрация работы библиотеки мониторинга");
    println!("=============================================");

    // Запускаем ресивер в параллельном потоке с каналом
    let receiver = MetricsReceiver::new("127.0.0.1:8080")?;
    let (_receiver_handle, metrics_rx) = receiver.start_with_channel();

    // Даём время ресиверу запуститься
    thread::sleep(Duration::from_millis(100));

    // Запускаем имитатор датчиков в отдельном потоке
    let sender_handle = thread::spawn(move || {
        let sender = MetricsSender::new("127.0.0.1:0").unwrap();
        println!("Имитатор датчиков запущен. Отправка данных каждую 1 секунду...");

        for i in 0..5 {
            let metrics = RoomMetrics::random();
            if let Err(e) = sender.send_to(&metrics, "127.0.0.1:8080") {
                eprintln!("Ошибка отправки: {}", e);
            } else {
                println!("[ДАТЧИК] Отправлен пакет {}", i + 1);
            }
            thread::sleep(Duration::from_secs(1));
        }
        println!("Имитатор датчиков завершил работу");
    });

    // Основной поток получает данные из канала
    println!("Основной поток ожидает данные...");

    let mut received_count = 0;
    while received_count < 5 {
        match metrics_rx.recv_timeout(Duration::from_secs(2)) {
            Ok((metrics, src_addr)) => {
                received_count += 1;
                println!(
                    "[ОСНОВНОЙ ПОТОК] Получено от {}: {:.1}°C, {:.1}% влажности, давление: {:.1}hPa, дверь: {}, воздух: {}",
                    src_addr,
                    metrics.temperature,
                    metrics.humidity,
                    metrics.pressure,
                    if metrics.door_open { "ОТКРЫТА" } else { "закрыта" },
                    metrics.air_quality
                );
            }
            Err(RecvTimeoutError::Timeout) => {
                println!("⏰ Тайм-аут ожидания данных...");
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => {
                println!("🔌 Канал закрыт");
                break;
            }
        }
    }

    // Ждём завершения потоков
    sender_handle.join().unwrap();

    println!("=============================================");
    println!("Демонстрация завершена успешно!");
    println!("Получено пакетов: {}", received_count);

    Ok(())
}
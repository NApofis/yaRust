// src/bins/monitor/monitor
// 
// use room_monitoring::MetricsReceiver;
// use room_monitoring::receiver::Receiver;
// use room_monitoring::receiver::MockReceiver;
// 
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let bind_addr = "127.0.0.1:8080";
// 
//     println!(" Запуск системы мониторинга банковского хранилища");
//     println!("Прослушивание адреса: {}", bind_addr);
//     println!("──────────────────────────────────────────────────");
// 
//     let receiver: Box<dyn Receiver> = if std::env::var("USE_MOCK").is_ok() {
//         Box::new(MockReceiver)
//     } else {
//         Box::new(MetricsReceiver::new(bind_addr)?)
//     };
//     let (receiver_handle, metrics_rx) = receiver.start_with_channel();
// 
//     println!("Система мониторинга запущена. Ожидание данных...");
//     println!("Нажмите Ctrl+C для остановки");
// 
//     let mut total_received = 0;
// 
//     // Основной цикл обработки данных
//     loop {
//         match metrics_rx.recv() {
//             Ok((metrics, _src_addr)) => {
//                 total_received += 1;
// 
//                 // Определяем статус тревоги
//                 let alert_status = if metrics.door_open {
//                     "🚨 ТРЕВОГА: ДВЕРЬ ОТКРЫТА!"
//                 } else if metrics.temperature > 30.0 {
//                     "⚠️  ВНИМАНИЕ: Высокая температура"
//                 } else if metrics.humidity > 70.0 {
//                     "⚠️  ВНИМАНИЕ: Высокая влажность"
//                 } else {
//                     "✅ Норма"
//                 };
// 
//                 println!(
//                     "[#{:03}] {} | Темп: {:.1}°C | Влажн: {:.1}% | Давл: {:.1}hPa | Дверь: {} | Воздух: {} | {}",
//                     total_received,
//                     metrics.formatted_time(),
//                     metrics.temperature,
//                     metrics.humidity,
//                     metrics.pressure,
//                     if metrics.door_open {
//                         "ОТКРЫТА"
//                     } else {
//                         "закрыта"
//                     },
//                     metrics.air_quality,
//                     alert_status
//                 );
//             }
//             Err(_) => {
//                 println!("🔌 Канал закрыт. Завершение работы.");
//                 break;
//             }
//         }
//     }
// 
//     // Пытаемся дождаться завершения потока
//     let _ = receiver_handle.join();
// 
//     println!("Итог: получено {} пакетов данных", total_received);
//     Ok(())
// }



use room_monitoring::{ConsoleLogger, Logger, MemoryLogger, MetricsReceiver};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = "127.0.0.1:8080";

    let console = Box::new(ConsoleLogger);
    let memory = Box::new(MemoryLogger::new());

    // Вектор трейтовых объектов
    let loggers: Vec<Box<dyn Logger>> = vec![console.clone(), memory];

    console.log(" Запуск системы мониторинга банковского хранилища");
    console.log(&format!("Прослушивание адреса: {}", bind_addr));
    console.log("──────────────────────────────────────────────────");

    let receiver = MetricsReceiver::new(bind_addr)?;
    let (receiver_handle, metrics_rx) = receiver.start_with_channel();

    console.log("Система мониторинга запущена. Ожидание данных...");
    console.log("Нажмите Ctrl+C для остановки");

    let mut total_received = 0;

    // Основной цикл обработки данных
    loop {
        match metrics_rx.recv() {
            Ok((metrics, _src_addr)) => {
                total_received += 1;

                // Определяем статус тревоги
                let alert_status = if metrics.door_open {
                    "🚨 ТРЕВОГА: ДВЕРЬ ОТКРЫТА!"
                } else if metrics.temperature > 30.0 {
                    "⚠️  ВНИМАНИЕ: Высокая температура"
                } else if metrics.humidity > 70.0 {
                    "⚠️  ВНИМАНИЕ: Высокая влажность"
                } else {
                    "✅ Норма"
                };

                for logger in &loggers {
                    logger.log(&format!(
                        "[#{:03}] {} | Темп: {:.1}°C | Влажн: {:.1}% | Давл: {:.1}hPa | Дверь: {} | {} | CO2 уровень: {:.2}| ",
                        total_received,
                        metrics.formatted_time(),
                        metrics.temperature,
                        metrics.humidity,
                        metrics.pressure,
                        if metrics.door_open {
                            "ОТКРЫТА"
                        } else {
                            "закрыта"
                        },
                        alert_status,
                        metrics.co2_level,

                    ));
                }
            }
            Err(_) => {
                console.log("Канал закрыт. Завершение работы.");
                break;
            }
        }
    }

    // Пытаемся дождаться завершения потока
    let _ = receiver_handle.join();
    for logger in &loggers {
        if let Some(mem) = logger.as_any().downcast_ref::<MemoryLogger>() {
            println!("Содержимое MemoryLogger:");
            for entry in mem.get_entries() {
                println!("  {entry}");
            }
        }
    }

    println!("Итог: получено {} пакетов данных", total_received);
    Ok(())
}
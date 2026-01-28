// examples/demo_features.rs

use room_monitoring::RoomMetrics;

fn main() {
    println!("Демонстрация работы features");
    println!("===============================");

    // Генерируем тестовые метрики
    let metrics = RoomMetrics::random();

    println!("Сгенерированные метрики:");
    println!("  Температура: {:.1}°C", metrics.temperature);
    println!("  Влажность: {:.1}%", metrics.humidity);
    println!("  Давление: {:.1}hPa", metrics.pressure);
    println!(
        "  Дверь: {}",
        if metrics.door_open {
            "открыта"
        } else {
            "закрыта"
        }
    );

    // Показываем, какие фичи активны
    #[cfg(feature = "random")]
    println!("\nФича 'random' активна");

    #[cfg(feature = "sqlite")]
    println!("Фича 'sqlite' активна");

    #[cfg(not(feature = "random"))]
    println!("\nФича 'random' отключена");

    #[cfg(not(feature = "sqlite"))]
    println!("Фича 'sqlite' отключена");

    // Демонстрация фичи sqlite
    #[cfg(feature = "sqlite")]
    {
        println!("\nSQL запрос:");
        println!("{}", metrics.to_sql());
    }
}
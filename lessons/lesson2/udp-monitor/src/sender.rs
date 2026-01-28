use crate::metrics::RoomMetrics;
use bincode;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

pub struct MetricsSender {
    socket: UdpSocket,
}

impl MetricsSender {
    pub fn new(bind_addr: &str) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        Ok(Self { socket })
    }


    // Метод отправки сообщений в сокет
    pub fn send_to(
        &self,
        metrics: &RoomMetrics,
        target_addr: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let encoded = bincode::serialize(metrics)?;
        self.socket.send_to(&encoded, target_addr)?;
        Ok(())
    }

    // Метод для запуска цикла постоянной отправки метрик
    pub fn start_broadcasting(
        self,
        target_addr: String,
        interval_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "Имитатор датчиков запущен. Отправка на {} каждые {}ms",
            target_addr, interval_ms
        );

        // Информация о включённых фичах
        #[cfg(feature = "random")]
        println!("✅ Фича 'random' активна - используется rand для генерации данных");

        #[cfg(not(feature = "random"))]
        println!("ℹ️  Фича 'random' отключена - используется детерминистическая генерация");


        loop {
            let metrics = RoomMetrics::random();

            match self.send_to(&metrics, &target_addr) {
                Ok(()) => {
                    println!(
                        "[{}] Отправлено: {:.1}C, {:.1}% влажности, давление: {:.1}hPa, дверь: {}, воздух: {}",
                        metrics.formatted_time(),
                        metrics.temperature,
                        metrics.humidity,
                        metrics.pressure,
                        if metrics.door_open {
                            "открыта"
                        } else {
                            "закрыта"
                        },
                        metrics.air_quality
                    );
                    
                    #[cfg(feature = "logging")]
                    {
                        println!("loginim");
                    }

                    // Демонстрация фичи sqlite
                    #[cfg(feature = "sqlite")]
                    {
                        println!("   💾 SQL: {}", metrics.to_sql());
                    }
                }
                Err(e) => {
                    eprintln!("Ошибка отправки: {}", e);
                }
            }

            thread::sleep(Duration::from_millis(interval_ms));
        }
    }
}
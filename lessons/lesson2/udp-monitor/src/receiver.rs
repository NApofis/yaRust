use crate::metrics::RoomMetrics;
use bincode;
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
pub struct MockReceiver;

pub struct MetricsReceiver {
    socket: UdpSocket,
}

impl MetricsReceiver {
    pub fn new(bind_addr: &str) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        println!("Ресивер запущен на {}", bind_addr);
        Ok(Self { socket })
    }

    // Старый метод для простого запуска
    pub fn start_in_thread(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            if let Err(e) = self.receive_loop() {
                eprintln!("Ошибка в receive_loop: {}", e);
            }
        })
    }

    // Метод с циклом для получения метрик
    pub fn receive_loop(self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = [0u8; 1024];

        println!("Ожидание данных...");

        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => match bincode::deserialize::<RoomMetrics>(&buf[..size]) {
                    Ok(metrics) => {
                        println!(
                            "[{}] Получено от {}: {:.1}C, {:.1}% влажности. Воздух {}",
                            metrics.formatted_time(),
                            src_addr,
                            metrics.temperature,
                            metrics.humidity,
                            metrics.air_quality
                        );
                        #[cfg(feature = "logging")]
                        {
                            println!("loginim");
                        }
                    }
                    Err(e) => {
                        eprintln!("Ошибка десериализации: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Ошибка получения данных: {}", e);
                }
            }
        }
    }

    // НОВЫЙ МЕТОД: запускает приём в отдельном потоке и возвращает канал для получения данных
    pub fn start_with_channel(
        self,
    ) -> (thread::JoinHandle<()>, mpsc::Receiver<(RoomMetrics, std::net::SocketAddr)>) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            if let Err(e) = self.receive_loop_with_channel(tx) {
                eprintln!("Ошибка в receive_loop_with_channel: {}", e);
            }
        });

        (handle, rx)
    }

    // Цикл приёма с отправкой в канал
    fn receive_loop_with_channel(
        self,
        tx: mpsc::Sender<(RoomMetrics, std::net::SocketAddr)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = [0u8; 1024];

        println!("Канал приёма данных активирован");

        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => {
                    match bincode::deserialize::<RoomMetrics>(&buf[..size]) {
                        Ok(metrics) => {
                            // Отправляем данные в основной поток
                            if tx.send((metrics, src_addr)).is_err() {
                                println!("Канал закрыт, завершение потока приёма");
                                break;
                            }
                            #[cfg(feature = "logging")]
                            {
                                println!("loginim");
                            }
                        }
                        Err(e) => {
                            eprintln!("Ошибка десериализации: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Ошибка получения данных: {}", e);
                }
            }
        }

        Ok(())
    }
}


use std::net::SocketAddr;

/// Общий интерфейс для всех приёмников метрик
pub trait Receiver: Send + Sync {
    fn start_with_channel(
        self: Box<Self>,
    ) -> (
        thread::JoinHandle<()>,
        mpsc::Receiver<(RoomMetrics, SocketAddr)>,
    );
}

impl Receiver for MetricsReceiver {
    fn start_with_channel(
        self: Box<Self>,
    ) -> (
        thread::JoinHandle<()>,
        mpsc::Receiver<(RoomMetrics, std::net::SocketAddr)>,
    ) {
        // просто вызываем уже реализованный метод
        MetricsReceiver::start_with_channel(*self)
    }
}


impl Receiver for MockReceiver {
    fn start_with_channel(
        self: Box<Self>,
    ) -> (
        thread::JoinHandle<()>,
        mpsc::Receiver<(RoomMetrics, std::net::SocketAddr)>,
    ) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            for i in 0..5 {
                let metrics = RoomMetrics {
                    temperature: 22.5 + i as f32,
                    humidity: 45.0,
                    pressure: 1013.0,
                    door_open: i % 2 == 0,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    air_quality: 1.0,
                };
                tx.send((metrics, "127.0.0.1:9999".parse().unwrap()))
                    .unwrap();
                thread::sleep(Duration::from_secs(1));
            }
        });

        (handle, rx)
    }
}
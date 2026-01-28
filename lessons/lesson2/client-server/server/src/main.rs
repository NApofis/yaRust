pub mod vault;
mod server;

use std::net::TcpListener;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use crate::server::handle_client;
use crate::vault::Vault;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    println!("Server listening on port 7878");

    let vault = Arc::new(Mutex::new(Vault::new(10))); // лимит 10 ячеек

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let vault = Arc::clone(&vault);
                thread::spawn(move || {
                    handle_client(stream, vault);
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}

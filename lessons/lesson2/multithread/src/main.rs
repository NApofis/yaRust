mod threads;
mod client_server;

fn main() {
    println!("----------------1 started----------------");
    threads::text_updater();
    println!("----------------2 started----------------");
    client_server::client_server();
    println!("----------------3 started----------------");

    println!("Hello, world!");
}

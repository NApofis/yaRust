use std::sync::{mpsc, Arc, RwLock, atomic::{AtomicUsize, Ordering}};
use std::thread;

#[derive(Debug)]
enum MessageKind {
    FinishAll,
    NewServer,
    Work{work_spec: String},
    ChangeEpoch{epoch: String},
}

fn serve (
    rx: mpsc::Receiver<(usize, MessageKind)>,
    id: usize,
    success_count: Arc<AtomicUsize>, // или другой тип
    epoch: Arc<RwLock<String>>,
)
{
    let current_epoch: Arc<RwLock<String>> = epoch.clone();
    while let Ok((from, msg_kind)) = rx.recv() {
        match msg_kind {
            MessageKind::FinishAll => break,
            MessageKind::NewServer => unreachable!("this message is not for server!"),
            MessageKind::ChangeEpoch { epoch: new_epoch } => {
                let mut guard = current_epoch.write().unwrap();
                *guard = new_epoch;
                println!("new epoch changed to {guard}");
            }
            MessageKind::Work { work_spec } => {
                println!( "worker-{} is being asked by client-{} at epoch '{}' to work '{}'",
                          id, from, epoch.read().unwrap(), work_spec );
            },
        }
        success_count.fetch_add(1, Ordering::Release);
    }
    println!("Finishing worker-{}", id);
}

fn balancer (
    rx: mpsc::Receiver<(usize, MessageKind)>,
    servers_count: usize,
    success_count: Arc<AtomicUsize>,
)
{
    fn make_and_append_server(
        all_servers: &mut Vec<(
            mpsc::Sender<(usize, MessageKind)>,
            thread::JoinHandle<()>
        )>,
        success_count: Arc<AtomicUsize>,
        epoch: Arc<RwLock<String>>,
    )
    {
        let canal = mpsc::channel();
        let thread = std::thread::spawn(move || serve(
            canal.1,
            success_count.load(Ordering::Acquire),
            success_count.clone(),
            epoch.clone()
        ));
        all_servers.push((canal.0, thread));
    }

    let epoch = Arc::new(RwLock::new("epoch-1".into()));
    let mut servers = Vec::new();
    for _ in 0..servers_count {
        make_and_append_server(&mut servers, success_count.clone(), epoch.clone());
    }
    let mut next_server = 0usize;
    while let Ok((from, msg_kind)) = rx.recv() {
        match msg_kind {
            MessageKind::FinishAll => {
                servers.drain(..).for_each(|s| {
                    s.0.send((from, MessageKind::FinishAll)).unwrap();
                    s.1.join().unwrap();
                });
                break;
            },
            MessageKind::NewServer => {
                make_and_append_server(&mut servers, success_count.clone(), epoch.clone());
            }
            MessageKind::Work { work_spec } => {
                servers[next_server].0.send((from, MessageKind::Work{work_spec})).unwrap();
            }
            MessageKind::ChangeEpoch { epoch } => {
                servers[next_server].0.send((from, MessageKind::ChangeEpoch{epoch: epoch.clone()})).unwrap();
            }
        }
        next_server = (next_server + 1).rem_euclid(servers.len());
    }
}

fn my_sleep() {
    thread::sleep(std::time::Duration::from_millis(100))
}

fn client1(server_tx: mpsc::Sender<(usize, MessageKind)>) {
    let id = 1;
    server_tx.send((id, MessageKind::Work{work_spec: "prepare".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-1".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-2".into()})).unwrap();
    my_sleep();
}
fn client2(server_tx: mpsc::Sender<(usize, MessageKind)>) {
    let id = 2;
    server_tx.send((id, MessageKind::Work{work_spec: "prepare".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::NewServer)).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-1".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-2".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-3".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-4".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-5".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-6".into()})).unwrap();
    my_sleep();
}
fn client3(server_tx: mpsc::Sender<(usize, MessageKind)>) {
    let id = 3;
    server_tx.send((id, MessageKind::Work{work_spec: "prepare".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-1".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::ChangeEpoch{epoch: "epoch-2".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-2".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-3".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-4".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-5".into()})).unwrap();
    my_sleep();
    server_tx.send((id, MessageKind::Work{work_spec: "work-6".into()})).unwrap();
    my_sleep();
}

pub fn client_server () {
    println!("Hello, world!");
    let (server_tx, server_rx) = mpsc::channel();
    let success_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

    // пригодится, чтобы переместить в поток к серверам
    let success_count_cloned = success_count.clone();

    let balancer_thread = std::thread::spawn(move || balancer(
        server_rx,
        1,
        success_count_cloned,
    ));

    std::thread::scope(|scope|{
        scope.spawn(|| client1(server_tx.clone()));
        scope.spawn(|| client2(server_tx.clone()));
        scope.spawn(|| client3(server_tx.clone()));
    });
    server_tx.send((0, MessageKind::FinishAll)).unwrap();
    balancer_thread.join().unwrap();


    println!("\nDone jobs = {}", success_count.load(Ordering::SeqCst));
}
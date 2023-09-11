use plain_state_machine_hype_train::State;
use std::io::Write;
use tokio::sync::{mpsc, oneshot};
use tracing::info;

pub enum Command {
    Ready(oneshot::Sender<String>),
    Store(oneshot::Sender<String>),
}

fn read_stdin_thread(tx: mpsc::Sender<Command>) {
    use std::io::BufRead;

    let mut lines = std::io::stdin().lock().lines();

    loop {
        print!("Please enter an operation: ready, store\n> ");
        std::io::stdout().lock().flush().unwrap();
        let line = lines.next().unwrap().unwrap();
        let (response_tx, response_rx) = oneshot::channel();
        let command = match line.as_str() {
            "ready" => Command::Ready(response_tx),
            "store" => Command::Store(response_tx),
            _ => {
                println!("unknown command, try again");
                continue;
            }
        };
        tx.blocking_send(command).unwrap();
        println!("{}", response_rx.blocking_recv().unwrap());
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    info!("Started operations");

    let (tx, mut rx) = mpsc::channel::<Command>(1);

    _ = std::thread::spawn(move || read_stdin_thread(tx));

    let mut state = State::default();
    while let Some(command) = rx.recv().await {
        let (tx, result) = match command {
            Command::Ready(tx) => (tx, state.ready()),
            Command::Store(tx) => (tx, state.store()),
        };
        state = match result {
            Ok(state) => {
                let _ = tx.send(format!("Transitioned to {}!", state.name()));
                state
            }
            Err(state) => {
                let _ = tx.send(format!(
                    "Transition failed! Current state is {}.",
                    state.name()
                ));
                state
            }
        }
    }
}

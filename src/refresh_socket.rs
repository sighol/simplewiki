use std::sync::{Arc, Mutex};
use std::{thread, time};

use ws::{self, CloseCode, Handler, Message, Result, Sender};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use dispatch;

use std::sync::mpsc;

struct Server {
    sender: Arc<Mutex<Sender>>,
    close_tx: mpsc::Sender<i32>,
}

impl Handler for Server {
    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        self.close_tx.send(0).unwrap();
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        println!("Server got message: '{}'", msg);
        let sender = self.sender.lock().unwrap();
        sender.send("Explicit message").unwrap();
        Ok(())
    }
}

type SendType = i32;
type Dispatcher = dispatch::SubscriptionHandler<SendType>;
type ArcDispatcher = Arc<Mutex<Dispatcher>>;

pub fn listen(port: u16, wiki_path: &str, verbose: bool) {
    let wiki_path = wiki_path.to_owned();

    let dispatcher = dispatch::SubscriptionHandler::new();
    let dispatcher = Arc::new(Mutex::new(dispatcher));

    start_file_watcher(dispatcher.clone(), wiki_path, verbose);
    start_ws(dispatcher.clone(), port);
}

fn start_file_watcher(dispatcher: ArcDispatcher, wiki_path: String, verbose: bool) {
    thread::spawn(move || {
        let (watcher_tx, watcher_rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher =
            Watcher::new(watcher_tx, time::Duration::from_millis(500)).expect("Create watcher");

        watcher.watch(wiki_path, RecursiveMode::Recursive).unwrap();

        loop {
            match watcher_rx.recv() {
                Ok(_event) => {
                    let mut dispatcher = dispatcher.lock().unwrap();
                    if verbose {
                        println!(
                            "Received file change event. Responding to {} subscribers",
                            dispatcher.len()
                        );
                    }
                    dispatcher.send_to_all(0);
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });
}

fn start_ws(dispatcher: ArcDispatcher, port: u16) {
    thread::spawn(move || {
        let addr = format!("127.0.0.1:{}", port);

        let dispatcher = dispatcher.clone();
        ws::listen(&addr, |out| {
            let recv = {
                let mut dispatcher = dispatcher.lock().unwrap();
                dispatcher.subscribe()
            };

            let sender_mutex = Arc::new(Mutex::new(out));

            let ws_sender = sender_mutex.clone();

            let (close_tx, close_rx) = mpsc::channel();
            let server = Server {
                sender: sender_mutex.clone(),
                close_tx: close_tx,
            };

            thread::spawn(move || loop {
                if let Ok(_) = close_rx.try_recv() {
                    return;
                }

                match recv.recv() {
                    Ok(_) => {
                        let sender = ws_sender.lock().unwrap();
                        sender.send("You need to refresh").unwrap();
                    }
                    Err(_) => {
                        return;
                    }
                }
            });

            return server;
        }).expect("Could not listen to web socket");
    });
}

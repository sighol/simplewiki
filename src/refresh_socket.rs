use std::{thread, time};
use std::sync::{Arc, Mutex};

use ws::{self, Handler, Message, Result, Handshake, CloseCode, Sender};

use spmc;
use notify::{RecommendedWatcher, Watcher, RecursiveMode};

use std::sync::mpsc;

struct Server {
    sender: Arc<Mutex<Sender>>,
    close_tx: mpsc::Sender<i32>,
}

impl Handler for Server {
    
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("New connection!");
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("Closing due to {}, code:{:?}", reason, code);
        self.close_tx.send(0).unwrap();
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        println!("Server got message: '{}'", msg);
        let sender = self.sender.lock().unwrap();
        sender.send("Explicit message").unwrap();
        Ok(())
    }
}

pub fn listen(port: i32, wiki_path: &str) {
    let (tx, rx) = spmc::channel();

    let wiki_path = wiki_path.to_owned();

    thread::spawn(move || {
        let (watcher_tx, watcher_rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher = Watcher::new(watcher_tx, time::Duration::from_secs(2)).expect("Create watcher");

        watcher.watch(wiki_path, RecursiveMode::Recursive).unwrap();

        loop {
            match watcher_rx.recv() {
                Ok(_event) =>{
                    tx.send(0).expect("Could not send to tx");
                } ,
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });

    thread::spawn(move || {
        let addr = format!("127.0.0.1:{}", port);
        ws::listen(&addr, |out| {
            let recv = rx.clone();

            let sender_mutex = Arc::new(Mutex::new(out));

            let ws_sender = sender_mutex.clone();

            let (close_tx, close_rx) = mpsc::channel();
            let server = Server {
                sender: sender_mutex.clone(),
                close_tx: close_tx,
            };

            thread::spawn(move || {
                let mut counter = 0;
                loop {
                    if let Ok(_) = close_rx.try_recv() {
                        return;
                    }

                    match recv.recv() {
                        Ok(_) => {
                            counter += 1;
                            println!("Refresh signal found: count={}", counter);
                            let sender = ws_sender.lock().unwrap();
                            sender.send("You need to refresh").unwrap();
                        },
                        Err(_) => {
                            println!("BREAKING OUT!");
                            return;
                        },
                    }
                }
            });

            return server;
        }).expect("Could not listen to web socket");
    });
}
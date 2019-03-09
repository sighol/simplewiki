use std::sync::{Arc, Mutex};
use std::{thread, time};

use ws::{self, CloseCode, Handler, Message, Result, Sender};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::broadcaster;

use std::sync::mpsc;

struct WebSocketConnection {
    /// Sending a message to this sender, will send to the browser.
    sender: Arc<Mutex<Sender>>,
    /// Send a message to close the web socket down
    close_s: mpsc::Sender<i32>,
}

impl Handler for WebSocketConnection {
    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        self.close_s.send(0).unwrap();
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        println!("Server got message: '{}'", msg);
        let sender = self.sender.lock().unwrap();
        sender.send("Explicit message").unwrap();
        Ok(())
    }
}

type SendType = i32;
type Broadcaster = broadcaster::Broadcaster<SendType>;
type ArcBroadcaster = Arc<Mutex<Broadcaster>>;

pub fn listen(port: u16, wiki_path: &str, verbose: bool) {
    let wiki_path = wiki_path.to_owned();

    let broadcaster = broadcaster::Broadcaster::new();
    let broadcaster = Arc::new(Mutex::new(broadcaster));

    start_file_watcher(broadcaster.clone(), wiki_path, verbose);
    start_ws(broadcaster.clone(), port);
}

fn start_file_watcher(broadcaster: ArcBroadcaster, wiki_path: String, verbose: bool) {
    thread::spawn(move || {
        let (watcher_s, watcher_r) = mpsc::channel();
        let mut watcher: RecommendedWatcher =
            Watcher::new(watcher_s, time::Duration::from_millis(500)).expect("Create watcher");

        watcher.watch(wiki_path, RecursiveMode::Recursive).unwrap();

        loop {
            match watcher_r.recv() {
                Ok(_event) => {
                    let mut broadcaster = broadcaster.lock().unwrap();
                    if verbose {
                        println!(
                            "Received file change event. Responding to {} subscribers",
                            broadcaster.len()
                        );
                    }
                    broadcaster.send_to_all(0);
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });
}

fn start_ws(dispatcher: ArcBroadcaster, port: u16) {
    thread::spawn(move || {
        let addr = format!("127.0.0.1:{}", port);

        let broadcaster = dispatcher.clone();
        ws::listen(&addr, |out| {
            let filewatcher_subscription = {
                let mut broadcaster = broadcaster.lock().unwrap();
                broadcaster.subscribe()
            };

            let sender_mutex = Arc::new(Mutex::new(out));

            let (close_s, close_r) = mpsc::channel();
            let websocket_connection = WebSocketConnection {
                sender: sender_mutex.clone(),
                close_s: close_s,
            };

            let ws_sender = sender_mutex.clone();
            thread::spawn(move || loop {
                if let Ok(_) = close_r.try_recv() {
                    return;
                }

                match filewatcher_subscription.recv() {
                    Ok(_) => {
                        let sender = ws_sender.lock().unwrap();
                        sender.send("You need to refresh").unwrap();
                    }
                    Err(_) => {
                        return;
                    }
                }
            });

            return websocket_connection;
        })
        .expect("Could not listen to web socket");
    });
}

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

pub struct SubscriptionHandler<T> where T: Send + Clone {
    subscribers: Arc<Mutex<Vec<Sender<T>>>>,
}

impl<T: Send + Clone> SubscriptionHandler<T> {

    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn send_to_all(&self, value: T) {
        let mut subscribers = self.subscribers.lock().unwrap();

        let start_len = subscribers.len();
        for index in (0..start_len).map(|i| start_len - 1 - i) {

            let should_remove = {
                let sub = &subscribers[index];
                sub.send(value.clone()).is_err()
            };

            if should_remove {
                subscribers.remove(index);
            }
        }
    }

    pub fn subscribe(&self) -> Receiver<T> {
        let subscribers = self.subscribers.clone();
        let mut subscribers = subscribers.lock().unwrap();
        let (tx, rx) = channel();
        subscribers.push(tx);
        rx
    }
}

#[cfg(test)]
mod tests {
    use *;

    #[test]
    fn it_works() {
        println!("Hello 1");

        let mut handler = SubscriptionHandler::new();

        let mut threads = vec![];

        for i in 0..10 {
            let t = thread::spawn(move || {
                let rx = handler.subscribe();
                loop {
                    if let Ok(next) = rx.recv() {
                        println!("Received {} on thread {}", next, i);
                    }
                }
            });

            threads.push(t);
        }

        thread::sleep(time::Duration::from_secs(1));

        handler.send_to_all(1337);

        for t in threads {
            t.join();
        }
    }
}
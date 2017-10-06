use std::sync::mpsc::{channel, Sender, Receiver};

pub struct SubscriptionHandler<T> where T: Send + Clone {
    subscribers: Vec<Sender<T>>,
}

impl<T: Send + Clone> SubscriptionHandler<T> {

    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
        }
    }

    pub fn send_to_all(&mut self, value: T) {
        let start_len = self.subscribers.len();
        for index in (0..start_len).map(|i| start_len - 1 - i) {

            let should_remove = {
                let sub = &self.subscribers[index];
                sub.send(value.clone()).is_err()
            };

            if should_remove {
                self.subscribers.remove(index);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.subscribers.len()
    }

    pub fn subscribe(&mut self) -> Receiver<T> {
        let (tx, rx) = channel();
        self.subscribers.push(tx);
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
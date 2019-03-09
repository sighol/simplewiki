use std::sync::mpsc::{channel, Receiver, Sender};

/// This broadcaster is a stupid implementation of a single producer, multiple
/// receiver broadcast. When the producer sends a message, it will be received
/// by all receivers.
pub struct Broadcaster<T>
where
    T: Clone,
{
    subscribers: Vec<Sender<T>>,
}

impl<T: Clone> Broadcaster<T> {
    pub fn new() -> Self {
        Broadcaster {
            subscribers: Vec::new(),
        }
    }

    /// Send message to all subscribers. If sending fails, remove the
    /// subscriber.
    pub fn send_to_all(&mut self, value: T) {
        let start_len = self.subscribers.len();
        // Loop backwards so that we can remove subscribers as we go
        for index in (0..start_len).map(|i| start_len - 1 - i) {
            let sub = &self.subscribers[index];
            if sub.send(value.clone()).is_err() {
                self.subscribers.remove(index);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.subscribers.len()
    }

    /// Subscribe to a channel. This will create a sender-receiver pair, put the
    /// sender into the broadcaster and return the receiver (a subscription).
    pub fn subscribe(&mut self) -> Receiver<T> {
        let (s, r) = channel();
        self.subscribers.push(s);
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time;

    #[test]
    fn it_works() {
        let handler = Broadcaster::new();
        let handler = Arc::new(Mutex::new(handler));

        let mut threads = vec![];

        for _ in 0..10 {
            let handler = handler.clone();
            let t = thread::spawn(move || {
                let subscription = {
                    let mut handler = handler.lock().unwrap();
                    handler.subscribe()
                };

                subscription.recv().unwrap();
            });

            threads.push(t);
        }

        // Sleep here so that all threads are up and running.
        thread::sleep(time::Duration::from_millis(100));

        let mut handler = handler.lock().unwrap();
        handler.send_to_all(1337);

        for t in threads {
            t.join().unwrap();
        }
    }
}

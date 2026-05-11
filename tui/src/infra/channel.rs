use std::fmt::Debug;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    Disconnected,
    Empty,
    Full,
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelError::Disconnected => write!(f, "channel disconnected"),
            ChannelError::Empty => write!(f, "channel empty"),
            ChannelError::Full => write!(f, "channel full"),
        }
    }
}

impl std::error::Error for ChannelError {}

pub trait Sender<T>: Clone + Send + 'static {
    fn send(&self, value: T) -> Result<(), ChannelError>;

    fn try_send(&self, value: T) -> Result<(), ChannelError>;
}

pub trait Receiver<T>: Send + 'static {
    fn recv(&self) -> Result<T, ChannelError>;

    fn try_recv(&self) -> Result<Option<T>, ChannelError>;
}

pub struct KanalSender<T>(kanal::Sender<T>);

impl<T> Clone for KanalSender<T> {
    fn clone(&self) -> Self {
        KanalSender(self.0.clone())
    }
}

impl<T: Send + 'static> Sender<T> for KanalSender<T> {
    fn send(&self, value: T) -> Result<(), ChannelError> {
        self.0.send(value).map_err(|_| ChannelError::Disconnected)
    }

    fn try_send(&self, value: T) -> Result<(), ChannelError> {
        match self.0.try_send(value) {
            Ok(true) => Ok(()),
            Ok(false) => Err(ChannelError::Full),
            Err(kanal::SendError::Closed | kanal::SendError::ReceiveClosed) => {
                Err(ChannelError::Disconnected)
            }
        }
    }
}

pub struct KanalReceiver<T>(kanal::Receiver<T>);

impl<T: Send + 'static> Receiver<T> for KanalReceiver<T> {
    fn recv(&self) -> Result<T, ChannelError> {
        self.0.recv().map_err(|_| ChannelError::Disconnected)
    }

    fn try_recv(&self) -> Result<Option<T>, ChannelError> {
        match self.0.try_recv() {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => Ok(None),
            Err(_) => Err(ChannelError::Disconnected),
        }
    }
}

pub fn unbounded<T: Send + 'static>() -> (impl Sender<T>, impl Receiver<T>) {
    let (tx, rx) = kanal::unbounded();
    (KanalSender(tx), KanalReceiver(rx))
}

pub fn bounded<T: Send + 'static>(capacity: usize) -> (impl Sender<T>, impl Receiver<T>) {
    let (tx, rx) = kanal::bounded(capacity);
    (KanalSender(tx), KanalReceiver(rx))
}

const BATCH_FLUSH_INTERVAL: Duration = Duration::from_millis(50);

pub struct BatchSender<T, S: Sender<Vec<T>>> {
    tx: S,
    buf: Vec<T>,
    capacity: usize,
    last_flush: Instant,
}

impl<T, S: Sender<Vec<T>>> BatchSender<T, S> {
    pub fn new(tx: S, capacity: usize) -> Self {
        Self {
            tx,
            buf: Vec::with_capacity(capacity),
            capacity,
            last_flush: Instant::now(),
        }
    }

    fn flush_buf(&mut self) {
        if !self.buf.is_empty() {
            let _ = self.tx.send(std::mem::replace(
                &mut self.buf,
                Vec::with_capacity(self.capacity),
            ));
            self.last_flush = Instant::now();
        }
    }

    pub fn push(&mut self, item: T) {
        self.buf.push(item);
        if self.buf.len() >= self.capacity || self.last_flush.elapsed() >= BATCH_FLUSH_INTERVAL {
            self.flush_buf();
        }
    }

    pub fn tick(&mut self) {
        if self.last_flush.elapsed() >= BATCH_FLUSH_INTERVAL {
            self.flush_buf();
        }
    }
}

impl<T, S: Sender<Vec<T>>> Drop for BatchSender<T, S> {
    fn drop(&mut self) {
        self.flush_buf();
    }
}

use std::marker::PhantomData;

pub struct MapSender<T, U, F, S> {
    tx: S,
    mapper: F,
    _phantom: PhantomData<fn(T) -> U>,
}

impl<T, U, F, S> MapSender<T, U, F, S> {
    pub fn new(tx: S, mapper: F) -> Self {
        Self {
            tx,
            mapper,
            _phantom: PhantomData,
        }
    }
}

impl<T, U, F, S> Clone for MapSender<T, U, F, S>
where
    S: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            mapper: self.mapper.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T, U, F, S> Sender<T> for MapSender<T, U, F, S>
where
    T: Send + 'static,
    U: Send + 'static,
    F: Fn(T) -> U + Clone + Send + 'static,
    S: Sender<U>,
{
    fn send(&self, value: T) -> Result<(), ChannelError> {
        self.tx.send((self.mapper)(value))
    }

    fn try_send(&self, value: T) -> Result<(), ChannelError> {
        self.tx.try_send((self.mapper)(value))
    }
}

pub type DefaultSender<T> = KanalSender<T>;

pub type DefaultReceiver<T> = KanalReceiver<T>;

pub fn unbounded_default<T: Send + 'static>() -> (DefaultSender<T>, DefaultReceiver<T>) {
    let (tx, rx) = kanal::unbounded();
    (KanalSender(tx), KanalReceiver(rx))
}

pub fn bounded_default<T: Send + 'static>(
    capacity: usize,
) -> (DefaultSender<T>, DefaultReceiver<T>) {
    let (tx, rx) = kanal::bounded(capacity);
    (KanalSender(tx), KanalReceiver(rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unbounded_send_recv() {
        let (tx, rx) = unbounded_default::<i32>();
        tx.send(42).unwrap();
        assert_eq!(rx.recv().unwrap(), 42);
    }

    #[test]
    fn test_try_recv_empty() {
        let (_tx, rx) = unbounded_default::<i32>();
        assert_eq!(rx.try_recv().unwrap(), None);
    }

    #[test]
    fn test_try_recv_with_value() {
        let (tx, rx) = unbounded_default::<i32>();
        tx.send(42).unwrap();
        assert_eq!(rx.try_recv().unwrap(), Some(42));
    }

    #[test]
    fn test_sender_clone() {
        let (tx, rx) = unbounded_default::<i32>();
        let tx2 = tx.clone();
        tx.send(1).unwrap();
        tx2.send(2).unwrap();
        assert_eq!(rx.recv().unwrap(), 1);
        assert_eq!(rx.recv().unwrap(), 2);
    }

    #[test]
    fn test_map_sender() {
        let (tx, rx) = unbounded_default::<String>();
        let map_tx = MapSender::new(tx, |i: i32| i.to_string());

        map_tx.send(42).unwrap();
        assert_eq!(rx.recv().unwrap(), "42");
    }
}

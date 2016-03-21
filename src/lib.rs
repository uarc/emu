extern crate oncemutex as om;
use std::io::Read;
use std::sync::mpsc::{SyncSender, Receiver, sync_channel};
use std::sync::{Mutex, Barrier};
use om::OnceMutex;

pub mod core0;

pub struct Permission {
    privilege: u8,
    address: u32,
}

pub struct Com<T> {
    pub permission: Permission,
    pub data: T,
}

impl<T> Com<T> {
    fn new(permission: Permission, data: T) -> Self {
        Com{
            permission: permission,
            data: data,
        }
    }
}

/// The emulated UARC synchronous bus
pub struct SenderBus<W> {
    // Send a stream to the target
    stream: SyncSender<Com<OnceMutex<Box<Read>>>>,
    // Incept the target
    incept: SyncSender<Com<(Permission, OnceMutex<Box<Read>>)>>,
    // Interrupt a target with a word
    send: SyncSender<Com<W>>,
    // Kill the target
    kill: SyncSender<Com<()>>,
}

impl<W> SenderBus<W> {
    /// Send a Read stream to the target through an Arc
    fn stream(&self, c: Com<OnceMutex<Box<Read>>>) {
        self.stream.send(c).ok().unwrap();
    }

    /// Send a Read stream and a fresh set of permissions through an Arc to incept a core
    fn incept(&self, c: Com<(Permission, OnceMutex<Box<Read>>)>) {
        self.incept.send(c).ok().unwrap();
    }

    /// Send a word to the target
    fn send(&self, c: Com<W>) {
        self.send.send(c).ok().unwrap();
    }

    /// Send a kill signal to the target
    fn kill(&self, c: Com<()>) {
        self.kill.send(c).ok().unwrap();
    }
}

/// The pair of the Bus which is defined to avoid duplication
pub struct ReceiverBus<W> {
    pub stream: Receiver<Com<OnceMutex<Box<Read>>>>,
    pub incept: Receiver<Com<(Permission, OnceMutex<Box<Read>>)>>,
    pub send: Receiver<Com<W>>,
    pub kill: Receiver<Com<()>>,
}

pub fn make_bus<W>() -> (SenderBus<W>, ReceiverBus<W>) {
    let streams = sync_channel(0);
    let incepts = sync_channel(0);
    let sends = sync_channel(0);
    let kills = sync_channel(0);

    (SenderBus{
        stream: streams.0,
        incept: incepts.0,
        send: sends.0,
        kill: kills.0,
    }, ReceiverBus{
        stream: streams.1,
        incept: incepts.1,
        send: sends.1,
        kill: kills.1,
    })
}

/// Core is a trait that cores implement to allow Bus to be created connecting cores
pub trait Core<W> {
    /// Set the internal buses
    fn append_buses<I>(&mut self, buses: I) where I: Iterator<Item=SenderBus<W>>;
    /// Aquire a bus located at a particular bus ID
    fn aquire_bus(&mut self, index: usize) -> SenderBus<W>;

    /// Begins operation in the current thread
    /// Killing the core will not end the thread.
    fn begin(&mut self);
}

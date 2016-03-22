extern crate oncemutex as om;
use std::sync::mpsc::{SyncSender, Receiver};

pub mod core0;

pub struct Permission {
    privilege: u8,
    address: u32,
}

pub struct Com<T> {
    pub permission: Permission,
    pub bus: usize,
    pub data: T,
}

impl<T> Com<T> {
    fn new(permission: Permission, bus: usize, data: T) -> Self {
        Com{
            permission: permission,
            bus: bus,
            data: data,
        }
    }
}

/// The emulated UARC synchronous bus
pub struct SenderBus<W> {
    // Associated bus ID we must send to the receiver
    pub bus: usize,
    // Send a stream to the target
    pub stream: SyncSender<Com<Receiver<W>>>,
    // Incept the target
    pub incept: SyncSender<Com<(Permission, Receiver<W>)>>,
    // Interrupt a target with a word
    pub send: SyncSender<Com<W>>,
    // Kill the target
    pub kill: SyncSender<Com<()>>,
}

impl<W> SenderBus<W> {
    /// Send a channel to the target
    fn stream(&self, permission: Permission, data: Receiver<W>) {
        self.stream.send(Com::new(permission, self.bus, data)).ok().unwrap();
    }

    /// Send a channel and a fresh set of permissions to incept a core
    fn incept(&self, permission: Permission, target_permission: Permission, instructions: Receiver<W>) {
        self.incept.send(Com::new(permission, self.bus, (target_permission, instructions))).ok().unwrap();
    }

    /// Send a word to the target
    fn send(&self, permission: Permission, value: W) {
        self.send.send(Com::new(permission, self.bus, value)).ok().unwrap();
    }

    /// Send a kill signal to the target
    fn kill(&self, permission: Permission) {
        self.kill.send(Com::new(permission, self.bus, ())).ok().unwrap();
    }
}

/// Core is a trait that cores implement to allow Bus to be created connecting cores
pub trait Core<W> {
    /// Set the internal buses
    fn append_sender(&mut self, sender: SenderBus<W>);
    /// Aquire a bus located at a particular bus ID
    fn aquire_sender(&mut self) -> SenderBus<W>;

    /// Begins operation in the current thread
    /// Killing the core will not end the thread.
    fn begin(&mut self);
}

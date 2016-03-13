extern crate nue;
use super::Core;
use std::io::Read;
use std::sync::{Mutex, Arc, Barrier};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

pub struct Core0<W> {
    running: bool,
    pc: W,
    dcs: [W; 4],
    carry: bool,
    overflow: bool,
    interrupt: bool,
    program: Vec<u8>,
    data: Vec<W>,

    //Synchronization for receiving interrupts
    interrupt_sender: Mutex<SyncSender<(W, Arc<Barrier>)>>,
    interrupt_receiver: Receiver<(W, Arc<Barrier>)>,
}

impl Core0<u32> {
    fn new(memory: usize) -> Self {
        let (sender, receiver) = sync_channel(0);
        Core0{
            running: false,
            pc: 0,
            dcs: [0; 4],
            carry: false,
            overflow: false,
            interrupt: false,
            program: Vec::new(),
            data: vec![0; memory],

            interrupt_sender: Mutex::new(sender),
            interrupt_receiver: receiver,
        }
    }
}

impl<W> Core<W> for Core0<W>
    where W: nue::Decode
{
    fn stream<R: Read>(&mut self, privilege: u8, address: u32, src: &mut R) {

    }
    fn incept<R: Read>(&mut self, privilege: u8, address: u32, src: &mut R) {

    }
    fn send(&mut self, privilege: u8, address: u32, value: W) {
        let mut guard = self.interrupt_sender.lock().ok().unwrap();
        let sender = &mut *guard;
        let barrier = Arc::new(Barrier::new(2));
        sender.send((value, barrier.clone()));
        barrier.wait();
    }
    fn kill(&mut self, privilege: u8, address: u32) {

    }
    fn is_running(&self) -> bool {
        false
    }
}

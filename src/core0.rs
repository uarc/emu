extern crate nue;
extern crate num;
use super::{Core, SenderBus, ReceiverBus, make_bus};
use std::io::Read;
use std::sync::{Mutex, Arc, Barrier};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::ops::Deref;

struct InBus<W> {
    bus: ReceiverBus<W>,
}

struct OutBus<W> {
    bus: SenderBus<W>,
}

#[derive(Default)]
struct DStack<W> {
    stack: Vec<W>,
}

impl<W> DStack<W> {
    fn rotate(&mut self, pos: u8) {
        let last = self.stack.len() - 1;
        // TODO: Introduce debug on out of range
        let v = self.stack.remove(last - pos as usize);
        self.stack.push(v);
    }

    fn replace<F>(&mut self, c: F) where F: FnOnce(W) -> W {
        let v = match self.stack.pop() {
            Some(v) => v,
            None => {
                // TODO: Add proper debugging here
                panic!("No value on stack to consume");
            },
        };

        self.stack.push(c(v));
    }
}

pub struct Core0<W> {
    running: bool,
    pc: W,
    dcs: [W; 4],
    carry: bool,
    overflow: bool,
    interrupt: bool,
    program: Vec<u8>,
    data: Vec<W>,

    // Incoming buses
    incoming_buses: Vec<InBus<W>>,
    outgoing_buses: Vec<OutBus<W>>,

    // Synchronization for receiving interrupts
    interrupt_sender: Mutex<SyncSender<(W, Arc<Barrier>)>>,
    interrupt_receiver: Receiver<(W, Arc<Barrier>)>,

    // Set up stack
    dstack: DStack<W>,
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

            incoming_buses: Vec::new(),
            outgoing_buses: Vec::new(),

            interrupt_sender: Mutex::new(sender),
            interrupt_receiver: receiver,

            dstack: DStack::default(),
        }
    }
}

impl<W> Core<W> for Core0<W>
    where W: Copy + num::Integer, usize: From<W>
{
    fn append_buses<I>(&mut self, buses: I) where I: Iterator<Item=SenderBus<W>> {

    }

    fn aquire_bus(&mut self, index: usize) -> SenderBus<W> {
        let buses = make_bus();
        self.incoming_buses.push(InBus{
            bus: buses.1
        });
        buses.0
    }

    fn begin(&mut self) {
        // Get disjoint references so borrows can occur simultaneously
        let dstack = &mut self.dstack;
        let data = &mut self.data;
        let prog = &mut self.program;
        let pc = &mut self.pc;
        let dcs = &mut self.dcs;
        // Poll for any sort of communication

        // Execute instruction
        match prog[usize::from(*pc)] {
            // rread#
            x @ 0...3 => {
                dstack.replace(|v| {
                    data[usize::from(dcs[x as usize] + v)]
                });
            },
            // TODO: Add all instructions
            _ => {},
        }
    }
}

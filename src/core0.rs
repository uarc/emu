extern crate num;
extern crate nue;
use super::{Com, Core, SenderBus, Permission};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

struct Bus<W> {
    sender: SenderBus<W>,
    selected: bool,
}

#[derive(Default)]
struct DStack<W> {
    stack: Vec<W>,
}

impl<W> DStack<W> where W: Copy {
    fn rotate(&mut self, pos: u8) {
        let last = self.stack.len() - 1;
        // TODO: Introduce debug on out of range
        let v = self.stack.remove(last - pos as usize);
        self.stack.push(v);
    }

    fn copy(&mut self, pos: u8) {
        let last = self.stack.len() - 1;
        // TODO: Introduce debug on out of range
        let v = self.stack[last - pos as usize];
        self.stack.push(v);
    }

    fn replace<F>(&mut self, c: F) where F: FnOnce(W) -> W {
        let v = match self.stack.pop() {
            Some(v) => v,
            None => {
                // TODO: Add proper debugging here
                panic!("core0: Attempted to consume a value when none was available.");
            },
        };

        self.stack.push(c(v));
    }
}

pub struct Core0<W> {
    permission: Permission,
    running: bool,
    pc: W,
    dcs: [W; 4],
    carry: bool,
    overflow: bool,
    interrupt: bool,
    program: Vec<u8>,
    data: Vec<W>,

    // Buses including senders
    buses: Vec<Bus<W>>,

    // Incoming streams
    incoming_streams: Vec<Receiver<Com<Receiver<W>>>>,

    // The channel that must be used to incept this core
    incept_channel: (SyncSender<Com<(Permission, Receiver<W>)>>, Receiver<Com<(Permission, Receiver<W>)>>),
    // The channel that must be used to send interrupts to this core
    send_channel: (SyncSender<Com<W>>, Receiver<Com<W>>),
    // The channel that must be used to kill this core
    kill_channel: (SyncSender<Com<()>>, Receiver<Com<()>>),

    // Set up stack
    dstack: DStack<W>,
}

impl Core0<u32> {
    fn new(memory: usize) -> Self {
        Core0{
            permission: Permission::default(),
            running: false,
            pc: 0,
            dcs: [0; 4],
            carry: false,
            overflow: false,
            interrupt: false,
            program: Vec::new(),
            data: vec![0; memory],

            incoming_streams: Vec::new(),
            buses: Vec::new(),

            incept_channel: sync_channel(0),
            send_channel: sync_channel(0),
            kill_channel: sync_channel(0),

            dstack: DStack::default(),
        }
    }
}

impl<W> Core<W> for Core0<W>
    where W: Copy + num::Integer + nue::Pod, usize: From<W>
{
    fn append_sender(&mut self, sender: SenderBus<W>) {
        self.buses.push(Bus{
            sender: sender,
            selected: false,
        });
    }

    fn aquire_sender(&mut self) -> SenderBus<W> {
        let stream_channel = sync_channel(0);
        self.incoming_streams.push(stream_channel.1);
        SenderBus{
            bus: self.incoming_streams.len() - 1,
            stream: stream_channel.0,
            incept: self.incept_channel.0.clone(),
            send: self.send_channel.0.clone(),
            kill: self.kill_channel.0.clone(),
        }
    }

    fn begin(&mut self) {
        assert_eq!(self.incoming_streams.len(), self.buses.len());
        // Get disjoint references so borrows can occur simultaneously
        let dstack = &mut self.dstack;
        let data = &mut self.data;
        let prog = &mut self.program;
        let pc = &mut self.pc;
        let dcs = &mut self.dcs;
        let permission = &mut self.permission;

        // Repeat loop of reinception perpetually
        loop {
            // Accept inception
            // TODO: Implement
            {
                let com = match self.incept_channel.1.recv() {
                    Ok(v) => v,
                    Err(_) => panic!("core0: Inception channel broken"),
                };

                *permission = com.data.0;
                let receiver = com.data.1;
                // Clear any previous program before loading the new one
                prog.clear();
                while let Ok(v) = receiver.recv() {
                    prog.extend_from_slice(v.as_slice());
                }
            }

            // Run until core is killed
            loop {
                // Poll for any sort of communication
                // TODO: Implement

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
    }
}

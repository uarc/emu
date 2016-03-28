extern crate num;
extern crate nue;
use super::{Com, Core, SenderBus, Permission};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::io::Read;
use std::collections::VecDeque;

struct Bus<W> {
    sender: SenderBus<W>,
    selected: bool,
    enabled: bool,
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

struct CStackElement<W> {
    pc: W,
    dcs: [W; 4],
    interrupt: bool,
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
    incoming_streams: Vec<Receiver<Com<Box<Read>>>>,

    // The channel that must be used to incept this core
    incept_channel: (SyncSender<Com<(Permission, Box<Read>)>>, Receiver<Com<(Permission, Box<Read>)>>),
    // The channel that must be used to send interrupts to this core
    send_channel: (SyncSender<Com<W>>, Receiver<Com<W>>),
    // The channel that must be used to kill this core
    kill_channel: (SyncSender<Com<()>>, Receiver<Com<()>>),

    dstack: DStack<W>,
    cstack: Vec<CStackElement<W>>,
    conveyor: VecDeque<W>,
}

impl<W> Core0<W>
    where W: num::PrimInt + Default
{
    pub fn new(memory: usize) -> Self {
        Core0{
            permission: Permission::default(),
            running: false,
            pc: W::zero(),
            dcs: [W::zero(); 4],
            carry: false,
            overflow: false,
            interrupt: false,
            program: Vec::new(),
            data: vec![W::zero(); memory],

            incoming_streams: Vec::new(),
            buses: Vec::new(),

            incept_channel: sync_channel(0),
            send_channel: sync_channel(0),
            kill_channel: sync_channel(0),

            dstack: DStack::default(),
            cstack: Vec::new(),
            conveyor: {
                use std::iter::repeat;
                let mut v = VecDeque::new();
                v.extend(repeat(W::zero()).take(16));
                v
            },
        }
    }
}

impl<W> Core<W> for Core0<W>
    where W: Copy + num::PrimInt + num::Signed + nue::Decode + nue::Encode, usize: From<W> + Into<W>
{
    fn append_sender(&mut self, sender: SenderBus<W>) {
        self.buses.push(Bus{
            sender: sender,
            selected: false,
            enabled: false,
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
        let running = &mut self.running;
        let dstack = &mut self.dstack;
        let cstack = &mut self.cstack;
        let conveyor = &mut self.conveyor;
        let buses = &mut self.buses;
        let data = &mut self.data;
        let prog = &mut self.program;
        let pc = &mut self.pc;
        let dcs = &mut self.dcs;
        let permission = &mut self.permission;
        let carry = &mut self.carry;
        let overflow = &mut self.overflow;
        let interrupt = &mut self.interrupt;

        let send_channel = &mut self.send_channel;

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
                let mut receiver = com.data.1;
                // Clear any previous program before loading the new one
                prog.clear();
                receiver.read_to_end(prog).expect("core0: Inception stream failed");
                *running = true;
            }

            // Run until core is killed
            loop {
                // Poll for any sort of communication
                // TODO: Implement

                // Execute instruction
                match prog[usize::from(*pc)] {
                    // rread#
                    x @ 0x00...0x03 => {
                        let select = x as usize;
                        dstack.replace(|v| {
                            data[usize::from(dcs[select] + v)]
                        });
                    },
                    // add#
                    x @ 0x04...0x07 => {
                        let select = x as usize - 0x04;
                        dstack.replace(|v| {
                            let dc_val = data[usize::from(dcs[select])];
                            let new = dc_val + v;
                            let old_signs = (v.is_negative(), dc_val.is_negative());
                            let new_sign = new < W::zero();
                            *overflow = if old_signs.0 != old_signs.1 {
                                false
                            } else {
                                new_sign != old_signs.0
                            };
                            *carry = old_signs.0 && old_signs.1 && !new_sign;
                            new
                        });
                    },
                    // inc
                    0x08 => {
                        dstack.replace(|v| {
                            let new = v + W::one();
                            let old_sign = v.is_negative();
                            let new_sign = new.is_negative();
                            // Going from positive to negative is overflow
                            *overflow = !old_sign && new_sign;
                            // If the new value wrapped back to zero we carry
                            *carry = new == W::zero();
                            new
                        });
                    },
                    // dec
                    0x09 => {
                        dstack.replace(|v| {
                            let new = v - W::one();
                            let old_sign = v.is_negative();
                            let new_sign = new.is_negative();
                            // Going from negative to positive is overflow
                            *overflow = old_sign && !new_sign;
                            // If the original value is zero, decrementing borrows
                            let borrow = v == W::zero();
                            *carry = !borrow;
                            new
                        });
                    },
                    // carry
                    0x0A => {
                        if *carry {
                            dstack.replace(|v| {
                                let new = v + W::one();
                                let old_sign = v.is_negative();
                                let new_sign = new.is_negative();
                                // Going from positive to negative is overflow
                                *overflow = !old_sign && new_sign;
                                // If the new value wrapped back to zero we carry
                                *carry = new == W::zero();
                                new
                            });
                        } else {
                            *overflow = false;
                            *carry = false;
                        }
                    },
                    // borrow
                    0x0B => {
                        if *carry {
                            *overflow = false;
                            *carry = true;
                        } else {
                            dstack.replace(|v| {
                                let new = v - W::one();
                                let old_sign = v.is_negative();
                                let new_sign = new.is_negative();
                                // Going from negative to positive is overflow
                                *overflow = old_sign && !new_sign;
                                // If the original value is zero, decrementing borrows
                                let borrow = v == W::zero();
                                *carry = !borrow;
                                new
                            });
                        }
                    },
                    // inv
                    0x0C => {
                        dstack.replace(|v| {
                            !v
                        });
                    },
                    // flush
                    0x0D => {},
                    // reads
                    0x0E => {
                        dstack.replace(|v| {
                            data[usize::from(v)]
                        });
                    },
                    // ret
                    0x0F => {
                        let elem = cstack.pop().expect("core0: Tried to return with empty return stack");
                        if elem.interrupt {
                            *interrupt = true;
                        }
                        *pc = elem.pc;
                        *dcs = elem.dcs;
                    },
                    // ien
                    0x10 => {
                        for bus in buses.iter_mut() {
                            bus.enabled |= bus.selected;
                        }
                    },
                    // idi
                    0x11 => {
                        for bus in buses.iter_mut() {
                            bus.enabled = false;
                        }
                    },
                    // recv
                    0x12 => {
                        let mut unaccepted = Vec::new();
                        // Wait for an enabled interrupt (place the rest in a vec to send them back)
                        let msg;
                        loop {
                            let v = send_channel.1.recv().expect("core0: Interrupt/send channel closed");
                            if buses[v.bus].enabled {
                                msg = v;
                                break;
                            }
                            unaccepted.push(v);
                        }
                        // Remove 2 values from back of conveyor
                        conveyor.pop_back();
                        conveyor.pop_back();
                        // Add the values to the conveyor in the correct order
                        conveyor.push_front(usize::into(msg.bus));
                        conveyor.push_front(msg.data);
                        // Send unaccepted messages back
                        for b in unaccepted {
                            send_channel.0.send(b).expect("core0: Interrupt/send channel closed");
                        }
                    },
                    // TODO: Add all instructions
                    _ => {},
                }
            }
        }
    }
}

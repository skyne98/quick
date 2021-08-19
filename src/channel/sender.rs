use std::{
    ffi::{CStr, CString},
    fs::File,
    os::raw::c_char,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use shmem_ipc::sharedring::{Receiver as ShmemReceiver, Sender as ShmemSender};

pub struct Sender {
    sender: ShmemSender<u8>,
}

impl Sender {
    pub fn new(capacity: u64, memfd: File, empty_signal: File, full_signal: File) -> Result<Self> {
        // Setup the ringbuffer.
        let sender = ShmemSender::open(capacity as usize, memfd, empty_signal, full_signal)?;
        Ok(Sender { sender })
    }

    pub fn send<S: AsRef<str>>(&mut self, msg: S) -> Result<()> {
        let msg = msg.as_ref();
        let msg_len = msg.len();
        let msg_len_with_meta = msg_len + 1;
        let mut data_sent = false;

        self.sender.send_raw(|p: *mut u8, mut count| unsafe {
            // No operation message
            if msg_len_with_meta > count {
                *p.offset(0 as isize) = 0;

                let bytes_to_fill = count as u64 - 1;
                if bytes_to_fill > 0 {
                    for i in 1..bytes_to_fill {
                        *p.offset(i as isize) = '\0' as u8;
                    }
                }
            } else {
                *p.offset(0 as isize) = 1;

                // Convert to bytes
                let data = msg.as_bytes();

                // Write bytes
                let ptr = p.offset(1 as isize);
                for i in 0..data.len() {
                    *ptr.offset(i as isize) = data[i];
                }

                data_sent = true;
                count = msg_len_with_meta;
            };

            count
        })?;

        if data_sent == false {
            self.send(msg)?;
        }

        Ok(())
    }
}

use std::{
    ffi::{CStr, CString},
    fs::File,
    os::raw::c_char,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use humansize::{file_size_opts, FileSize};
use shmem_ipc::sharedring::Receiver;

use crate::CAPACITY;

pub struct QuickServer {
    receiver: Receiver<u8>,
}

impl QuickServer {
    pub fn new() -> Result<(Self, File, File, File, u64)> {
        println!(
            "Buffer: {}",
            CAPACITY.file_size(file_size_opts::DECIMAL).unwrap()
        );

        // Create a receiver in shared memory.
        let receiver = Receiver::new(CAPACITY as usize)?;
        let mem_fd = receiver.memfd().as_file().try_clone()?;
        let empty_signal = receiver.empty_signal().try_clone()?;
        let full_signal = receiver.full_signal().try_clone()?;

        Ok((
            QuickServer { receiver },
            mem_fd,
            empty_signal,
            full_signal,
            CAPACITY as u64,
        ))
    }

    pub fn receive(&mut self) -> Result<Option<&str>> {
        let mut result_data = None;

        self.receiver.block_until_readable()?;
        self.receiver.receive_raw(|ptr: *const u8, count| unsafe {
            let op = *ptr.offset(0 as isize);
            let ptr = ptr.offset(1 as isize);

            if op == 1 {
                // To byte array
                let data = std::slice::from_raw_parts(ptr, count - 1);

                // To string
                let data_str = std::str::from_utf8(data).unwrap();
                result_data = Some(data_str);
            }

            count
        })?;

        Ok(result_data)
    }
}

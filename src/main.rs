use std::sync::{Arc, Mutex};

use anyhow::Result;
use humansize::{file_size_opts as options, FileSize};
use lipsum::lipsum;
use mimalloc::MiMalloc;
use nng::*;
use shared::{client::QuickClient, server::QuickServer};
use shmem_ipc::sharedring::{Receiver, Sender};

mod shared;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const ADDRESS: &'static str = "ipc://socket";
// const ADDRESS: &'static str = "ws://localhost:9999";
// const ADDRESS: &'static str = "inproc://test";
// const ADDRESS: &'static str = "tcp://localhost:9999";
const ITERATIONS: u64 = 10_000;
const IPSUM_SIZE: u64 = 1_000;
const CAPACITY: usize = 50_000_000;

#[tokio::main]
async fn main() -> Result<()> {
    let text = lipsum(IPSUM_SIZE as usize);

    //nng_benchmark(&text)?;
    shmem_benchmark(&text)?;

    Ok(())
}

fn nng_benchmark(text: &str) -> nng::Result<()> {
    let text_len = text.len();
    println!(
        "Message size: {}",
        text_len.file_size(options::DECIMAL).unwrap()
    );

    let server = Socket::new(Protocol::Rep0).unwrap();
    server.listen(ADDRESS).unwrap();
    std::thread::spawn(move || {
        println!("Starting server thread");
        loop {
            let msg = server.recv().unwrap();
            server.send(msg).unwrap();
        }
    });

    println!("Starting client");
    let client = Socket::new(Protocol::Req0)?;
    client.dial(ADDRESS)?;

    let start_time = std::time::Instant::now();
    for i in 0..ITERATIONS {
        client.send(text.as_bytes())?;
        client.recv()?;
    }
    let duration = std::time::Instant::now().duration_since(start_time);
    println!(
        "Time to exchange {} messages: {}",
        ITERATIONS,
        humantime::format_duration(duration.clone())
    );
    let duration_per_message =
        std::time::Duration::from_nanos(duration.as_nanos() as u64 / ITERATIONS);
    println!(
        "Time per message: {}",
        humantime::format_duration(duration_per_message.clone())
    );

    let messages_per_ns = ITERATIONS as f64 / duration.as_nanos() as f64;
    let messages_per_sec = messages_per_ns * 1000.0 /* us */ * 1000.0 /* ms */ * 1000.0 /* sec */;
    let messages_per_sec = messages_per_sec * 2.0; // because client -> server, then server -> client
    println!("Messages per second: {}", messages_per_sec);

    let throughput_ns = (ITERATIONS * text_len as u64) as f64 / duration.as_nanos() as f64;
    let throughput = throughput_ns * 1000.0 /* us */ * 1000.0 /* ms */ * 1000.0 /* sec */;
    let throughput = throughput as u64;
    let throughput = throughput * 2; // because client -> server, then server -> client
    println!(
        "Throughput: {}/s",
        throughput.file_size(options::DECIMAL).unwrap()
    );

    Ok(())
}

fn shmem_benchmark(text: &str) -> Result<()> {
    let (mut server, mem_fd, empty_fignal, full_signal, capacity) = QuickServer::new()?;
    let mut client = QuickClient::new(capacity, mem_fd, empty_fignal, full_signal)?;

    let text_len = text.len();
    println!(
        "Message size: {}",
        text_len.file_size(options::DECIMAL).unwrap()
    );

    let received = Arc::new(Mutex::new(0u64));
    let received_for_thread = received.clone();
    std::thread::spawn(move || {
        println!("Starting server thread");
        loop {
            if let Some(_) = server.receive().unwrap() {
                *received_for_thread.lock().unwrap() += 1;
            }
        }
    });

    let start_time = std::time::Instant::now();
    for i in 0..ITERATIONS {
        client.send(&text)?;
    }

    // Wait until everything is sent
    loop {
        if *received.lock().unwrap() == ITERATIONS {
            break;
        }
    }

    let duration = std::time::Instant::now().duration_since(start_time);
    println!(
        "Time to exchange {} messages: {}",
        ITERATIONS,
        humantime::format_duration(duration.clone())
    );
    let duration_per_message =
        std::time::Duration::from_nanos(duration.as_nanos() as u64 / ITERATIONS);
    println!(
        "Time per message: {}",
        humantime::format_duration(duration_per_message.clone())
    );

    let messages_per_ns = ITERATIONS as f64 / duration.as_nanos() as f64;
    let messages_per_sec = messages_per_ns * 1000.0 /* us */ * 1000.0 /* ms */ * 1000.0 /* sec */;
    println!("Messages per second: {}", messages_per_sec);

    let throughput_ns = (ITERATIONS * text_len as u64) as f64 / duration.as_nanos() as f64;
    let throughput = throughput_ns * 1000.0 /* us */ * 1000.0 /* ms */ * 1000.0 /* sec */;
    let throughput = throughput as u64;
    println!(
        "Throughput: {}/s",
        throughput.file_size(options::DECIMAL).unwrap()
    );

    Ok(())
}

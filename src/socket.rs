use std::{os::unix::prelude::RawFd, path::Path};

use anyhow::{Context, Result};
use tokio_compat_02::FutureExt;
use uds::{
    tokio::{UnixSeqpacketConn, UnixSeqpacketListener},
    UnixSocketAddr,
};

pub struct Receiver {
    listener: UnixSeqpacketListener,
}

impl Receiver {
    pub async fn new<P: AsRef<Path>>(address: P) -> Result<Self> {
        async {
            let address = address.as_ref();
            let socket_address =
                UnixSocketAddr::from_path(address).context("Cannot create a socket address")?;
            let listener = UnixSeqpacketListener::bind_addr(&socket_address)
                .context("Cannot open a listener")?;

            Ok(Receiver { listener })
        }
        .compat()
        .await
    }

    pub async fn accept(&mut self) -> Result<Peer> {
        async {
            let (connection, address) = self.listener.accept().await?;
            Ok(Peer::new(address, connection))
        }
        .compat()
        .await
    }
}

pub struct Peer {
    address: UnixSocketAddr,
    connection: UnixSeqpacketConn,
}

impl Peer {
    pub fn new(address: UnixSocketAddr, connection: UnixSeqpacketConn) -> Self {
        Peer {
            address,
            connection,
        }
    }
    pub async fn sender<P: AsRef<Path>>(address: P) -> Result<Self> {
        async {
            let address = address.as_ref();
            let socket_address =
                UnixSocketAddr::from_path(address).context("Cannot create a socket address")?;
            let connection = UnixSeqpacketConn::connect_addr(&socket_address)
                .await
                .context("Cannot connect to a socket")?;

            Ok(Peer {
                address: socket_address,
                connection,
            })
        }
        .compat()
        .await
    }

    // Normal data
    pub async fn send<N: AsRef<[u8]>>(&mut self, data: N) -> Result<()> {
        async {
            let data = data.as_ref();
            let data_len = data.len() as u32;

            self.connection.send(&data_len.to_le_bytes()[..]).await?;
            self.connection.send(data).await?;
            Ok(())
        }
        .compat()
        .await
    }
    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        async {
            let buffer = &mut [0u8; 4];
            self.connection.recv(buffer).await?;
            let data_len = u32::from_le_bytes(*buffer);

            let mut buffer = vec![0u8; data_len as usize];
            self.connection.recv(&mut buffer[..]).await?;

            Ok(buffer.into_iter().collect())
        }
        .compat()
        .await
    }

    // File descriptors
    pub async fn send_fds<F: AsRef<[RawFd]>>(&mut self, fds: F) -> Result<()> {
        async {
            let fds = fds.as_ref();
            let data = fds.len() as u32;

            self.connection.send(&data.to_le_bytes()[..]).await?;
            self.connection.send_fds(&[], fds).await?;
            Ok(())
        }
        .compat()
        .await
    }
    pub async fn receive_fds(&mut self) -> Result<Vec<RawFd>> {
        async {
            let buffer = &mut [0u8; 4];
            self.connection.recv(buffer).await?;
            let fds_len = u32::from_le_bytes(*buffer);

            let mut buffer = vec![-1; fds_len as usize];
            self.connection
                .recv_fds(&mut [0u8; 1], &mut buffer[..])
                .await?;

            Ok(buffer.into_iter().collect())
        }
        .compat()
        .await
    }
}

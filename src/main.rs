use async_std::net::UdpSocket;
use async_std::stream::{Stream, StreamExt};
use async_std::task;
use futures::future::Future;
use futures::ready;
use std::io::Result;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<()> {
    udp_test().await
}

async fn udp_test() -> Result<()> {
    let addr1 = "127.0.0.1:8080";
    let addr2 = "127.0.0.1:8081";
    let echo_task = task::spawn(read_loop(addr1));
    let send_task = task::spawn(send_loop(addr2, addr1));
    send_task.await?;
    echo_task.await?;
    Ok(())
}

async fn read_loop(bind_addr: &str) -> Result<()> {
    let socket = UdpSocket::bind(bind_addr).await?;
    let local_addr = socket.local_addr().unwrap();
    let mut stream = UdpStream::new(&socket);
    while let Some(message) = stream.next().await {
        let message = message?;
        eprintln!(
            "[{}] recv from {}: {:?}",
            local_addr,
            message.1,
            String::from_utf8(message.0).unwrap()
        );
    }
    Ok(())
}

async fn send_loop(bind_addr: &str, peer_addr: &str) -> Result<()> {
    let socket = UdpSocket::bind(bind_addr).await?;
    let local_addr = socket.local_addr().unwrap();
    let mut i = 0;
    loop {
        i += 1;
        let string = format!("hello {}", i).to_string();
        let buf = string.as_bytes();
        socket.send_to(&buf[..], &peer_addr).await?;
        eprintln!("[{}] sent to {}: {:?}", local_addr, peer_addr, string);
        task::sleep(Duration::from_secs(1)).await;
    }
}

pub struct UdpStream<'a> {
    socket: &'a UdpSocket,
    fut: Option<Pin<Box<dyn Future<Output = Result<(Vec<u8>, SocketAddr)>> + Send + Sync + 'a>>>,
    // buf: Option<Vec<u8>>,
}

impl<'a> UdpStream<'a> {
    pub fn new(socket: &'a UdpSocket) -> Self {
        // let buf = vec![0u8; 1024];
        Self {
            socket,
            fut: None,
            // buf: Some(buf),
        }
    }

    pub fn get_ref(&self) -> &'a UdpSocket {
        self.socket
    }
}

async fn recv_next(socket: &UdpSocket) -> Result<(Vec<u8>, SocketAddr)> {
    let mut buf = vec![0u8; 1024];
    let res = socket.recv_from(&mut buf).await;
    match res {
        Err(e) => Err(e),
        Ok((n, addr)) => Ok((buf[..n].to_vec(), addr)),
    }
}

impl<'a> Stream for UdpStream<'a> {
    type Item = Result<(Vec<u8>, SocketAddr)>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if self.fut.is_none() {
                let fut = recv_next(self.socket);
                self.fut = Some(Box::pin(fut));
            }

            if let Some(f) = &mut self.fut {
                let res = ready!(f.as_mut().poll(cx));
                self.fut = None;
                return match res {
                    Err(e) => Poll::Ready(Some(Err(e))),
                    Ok((buf, addr)) => Poll::Ready(Some(Ok((buf, addr)))),
                };
            }
        }
    }
}

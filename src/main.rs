use async_std::net::UdpSocket;
use async_std::stream::{Stream, StreamExt};
use async_std::task;
use futures::future::Future;
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
    let mut stream = UdpStream::new(socket);
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

type ReceiveResult = Result<(usize, SocketAddr)>;

struct UdpStream {
    inner: Option<(UdpSocket, Vec<u8>)>,
    fut: Option<Pin<Box<dyn Future<Output = (UdpSocket, Vec<u8>, ReceiveResult)> + Send + Sync>>>,
}

impl UdpStream {
    pub fn new(socket: UdpSocket) -> Self {
        let buf = vec![0u8; 1024];
        Self {
            fut: None,
            inner: Some((socket, buf)),
        }
    }
}
impl Stream for UdpStream {
    type Item = Result<(Vec<u8>, SocketAddr)>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut fut = if let Some(fut) = self.fut.take() {
            fut
        } else {
            if let Some((socket, buf)) = self.inner.take() {
                let fut = recv_next(socket, buf);
                Box::pin(fut)
            } else {
                unreachable!()
            }
        };

        let res = Pin::new(&mut fut).poll(cx);

        match res {
            Poll::Pending => {
                self.fut = Some(fut);
                Poll::Pending
            }
            Poll::Ready((socket, buf, res)) => match res {
                Ok((n, peer_addr)) => {
                    let vec = buf[..n].to_vec();
                    self.inner = Some((socket, buf));
                    Poll::Ready(Some(Ok((vec, peer_addr))))
                }
                Err(e) => Poll::Ready(Some(Err(e))),
            },
        }
    }
}

async fn recv_next(socket: UdpSocket, mut buf: Vec<u8>) -> (UdpSocket, Vec<u8>, ReceiveResult) {
    let res = socket.recv_from(&mut buf).await;
    (socket, buf, res)
}

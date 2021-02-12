use async_std::net::UdpSocket;
use async_std::stream::{Stream, StreamExt};
use async_std::task;
use futures::future::Future;
use futures::pin_mut;
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

struct UdpStream {
    socket: UdpSocket,
    buf: Vec<u8>,
}

impl UdpStream {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            buf: vec![0u8; 1024],
        }
    }
}

impl Stream for UdpStream {
    type Item = Result<(Vec<u8>, SocketAddr)>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let res = {
            let fut = this.socket.recv_from(&mut this.buf);
            pin_mut!(fut);
            fut.poll(cx)
        };
        match res {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(res)) => {
                let buf = this.buf[..res.0].to_vec();
                let peer = res.1;
                Poll::Ready(Some(Ok((buf, peer))))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
        }
    }
}

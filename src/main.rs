use futures::future::join_all;
use tokio::{
    net::{TcpListener, TcpStream},
    time::Instant,
};

const LOCALHOST: &str = "127.0.0.1";

struct Proxy {
    from: u16,
    to: u16,
    listener: TcpListener,
}

impl Proxy {
    async fn new(from: u16, to: u16) -> tokio::io::Result<Self> {
        Ok(Self {
            from,
            to,
            listener: TcpListener::bind((LOCALHOST, from)).await?,
        })
    }

    async fn accept(&mut self) -> tokio::io::Result<(TcpStream, std::net::SocketAddr)> {
        self.listener.accept().await
    }
}

async fn redirect(mut socket: TcpStream, from: u16, to: u16) -> tokio::io::Result<()> {
    let instant = Instant::now();

    let mut client = TcpStream::connect((LOCALHOST, to)).await?;
    let _ = tokio::io::copy_bidirectional(&mut client, &mut socket).await;

    let elapsed = instant.elapsed();

    println!(":{from} -> :{to} in {elapsed:?}");

    Ok(())
}

async fn accept_loop(mut proxy: Proxy) {
    loop {
        if let Ok((socket, _)) = proxy.accept().await {
            tokio::spawn(redirect(socket, proxy.from, proxy.to));
        }
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    println!("Stating proxy...");

    let proxis = join_all(vec![Proxy::new(8080, 3000)])
        .await
        .into_iter()
        .flatten()
        .map(|proxy| tokio::spawn(accept_loop(proxy)));

    let _ = join_all(proxis).await;

    Ok(())
}

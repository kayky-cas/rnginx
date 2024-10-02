use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use clap::Parser;
use futures::future::join_all;
use tokio::{
    net::{TcpListener, TcpStream},
    time::Instant,
};

const LOCALHOST: &str = "127.0.0.1";
const DELIMITER: &str = "->";

struct Proxy {
    from: u16,
    to: u16,
}

impl Proxy {
    fn new(from: u16, to: u16) -> Self {
        Self { from, to }
    }
}

async fn redirect(mut socket: TcpStream, from: u16, to: u16) -> tokio::io::Result<()> {
    let instant = Instant::now();

    let mut client = TcpStream::connect((LOCALHOST, to)).await.map_err(|err| {
        eprintln!(":{} is off.", to);
        err
    })?;

    let _ = tokio::io::copy_bidirectional(&mut client, &mut socket).await;

    let elapsed = instant.elapsed();

    println!(":{from} -> :{to} in {elapsed:?}");

    Ok(())
}

async fn accept_loop(proxy: Proxy) -> tokio::io::Result<()> {
    let listener = TcpListener::bind((LOCALHOST, proxy.from)).await?;

    loop {
        if let Ok((socket, _)) = listener.accept().await {
            tokio::spawn(redirect(socket, proxy.from, proxy.to));
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    println!("Stating proxy...");

    let args = Args::parse();

    let file_name = args.file.unwrap_or(PathBuf::from("rngix.conf"));

    let file = File::open(file_name)?;

    let proxis = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .flat_map(|line| {
            let (from, to) = line.split_once(DELIMITER)?;
            let from = from.trim().parse().ok()?;
            let to = to.trim().parse().ok()?;
            Some(Proxy::new(from, to))
        })
        .map(|proxy| tokio::spawn(accept_loop(proxy)));

    let _ = join_all(proxis).await;

    Ok(())
}

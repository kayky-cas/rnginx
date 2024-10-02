use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::Arc,
};

use clap::Parser;
use futures::future::join_all;

use tokio::{
    net::{TcpListener, TcpStream},
    time::Instant,
};

const LOCALHOST: &str = "127.0.0.1";
const DELIMITER: &str = "->";

async fn redirect(mut socket: TcpStream, from: u16, to: Arc<[u16]>) -> tokio::io::Result<()> {
    let instant = Instant::now();

    let streams: Vec<_> = to
        .iter()
        .map(|to| (to, TcpStream::connect((LOCALHOST, *to))))
        .collect();

    for (to, stream) in streams {
        let mut client = match stream.await {
            Ok(client) => client,
            Err(err) => {
                println!(":{from} -> :{to} failed: {err}");
                continue;
            }
        };

        let _ = tokio::io::copy_bidirectional(&mut client, &mut socket).await;

        let elapsed = instant.elapsed();

        println!(":{from} -> :{to} in {elapsed:?}");

        break;
    }

    Ok(())
}

async fn accept_loop(from: u16, to: Vec<u16>) -> tokio::io::Result<()> {
    let listener = TcpListener::bind((LOCALHOST, from)).await?;

    let to: Arc<[u16]> = to.into();

    loop {
        if let Ok((socket, _)) = listener.accept().await {
            tokio::spawn(redirect(socket, from, to.clone()));
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

    let mut proxy_table: HashMap<u16, Vec<u16>> = HashMap::new();

    for (from, to) in BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .flat_map(|line| {
            let (from, to) = line.trim().split_once(DELIMITER)?;
            let from = from.trim_end().parse().ok()?;
            let to = to.trim_start().parse().ok()?;
            Some((from, to))
        })
    {
        proxy_table
            .entry(from)
            .and_modify(|v| v.push(to))
            .or_insert(vec![to]);
    }

    let proxis = proxy_table
        .into_iter()
        .map(|(from, to)| tokio::spawn(accept_loop(from, to)));

    let _ = join_all(proxis).await;

    Ok(())
}

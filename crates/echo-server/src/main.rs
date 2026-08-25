#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "this is a cli app"
)]

use clap::{Parser, ValueEnum};
use std::io::Result;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, UdpSocket};

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum Protocol {
    Udp,
    Tcp,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "TCP/UDP Echo Server", long_about = None)]
struct Args {
    /// Transport protocol to use (udp or tcp)
    #[arg(value_enum)]
    protocol: Protocol,

    /// Port to listen on (e.g. 8080)
    port: u16,

    /// IP address to bind to [default: 0.0.0.0]
    #[arg(default_value = "0.0.0.0")]
    address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let bind_addr = format!("{}:{}", args.address, args.port);

    match args.protocol {
        Protocol::Udp => run_udp_server(&bind_addr).await?,
        Protocol::Tcp => run_tcp_server(&bind_addr).await?,
    }

    Ok(())
}

async fn run_udp_server(addr: &str) -> Result<()> {
    let socket = UdpSocket::bind(addr).await?;
    println!("UDP Echo Server listening on {addr}");

    let mut buf = [0u8; 1024];

    loop {
        let (len, peer) = socket.recv_from(&mut buf).await?;
        let msg = String::from_utf8_lossy(&buf[..len]);
        println!("[UDP] Received from {}: {}", peer, msg.trim_end());

        socket.send_to(&buf[..len], peer).await?;
    }
}

async fn run_tcp_server(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("TCP Echo Server listening on {addr}");

    loop {
        let (mut socket, peer) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                println!("[TCP] {peer} connected");
                match socket.read(&mut buf).await {
                    Ok(0) => {
                        println!("[TCP] {peer} disconnected");
                        break; // Connection closed cleanly
                    }
                    Ok(len) => {
                        let msg = String::from_utf8_lossy(&buf[..len]);
                        println!("[TCP] Received from {}: {}", peer, msg.trim_end());

                        if socket.write_all(&buf[..len]).await.is_err() {
                            eprintln!("[TCP] Error occurred when responding");
                            break;
                        }
                    }
                    Err(err_msg) => {
                        eprintln!("[TCP] Error occurred while reading message: {err_msg}");
                        break;
                    }
                }
            }
        });
    }
}

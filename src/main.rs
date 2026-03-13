use std::net::{SocketAddr, UdpSocket, ToSocketAddrs};
use std::time::{Duration, Instant};
use sysinfo::System;
use stunclient::StunClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Kernel version
    let kernel_version = System::kernel_version().unwrap_or(String::from("unknown kernel version"));
    println!("Kernel version: {}", kernel_version);

    // Bind local UDP socket
    let local_socket = UdpSocket::bind("0.0.0.0:0")?;
    local_socket.set_read_timeout(Some(Duration::from_millis(500)))?;

    // Resolve STUN server
    let stun_server = "stun.l.google.com:19302"
        .to_socket_addrs()?
        .next()
        .ok_or("Failed to resolve STUN server")?;
    let client = StunClient::new(stun_server);

    // Discover public endpoint
    let mapped_addr = client.query_external_address(&local_socket)?;
    println!("Public endpoint discovered via STUN: {}", mapped_addr);

    // Input peer address
    println!("Enter peer's public IP:Port (e.g., 203.0.113.5:54321):");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let peer_addr: SocketAddr = input.trim().parse()?;

    let message = format!("Hello from kernel {}", kernel_version);
    let start = Instant::now();

    let mut buf = [0u8; 1024];
    println!("Starting hole punching. Press Ctrl+C to exit.");

    loop {
        // Send a packet every 500ms
        local_socket.send_to(message.as_bytes(), peer_addr)?;

        // Try to receive
        match local_socket.recv_from(&mut buf) {
            Ok((n, addr)) => {
                let received = String::from_utf8_lossy(&buf[..n]);
                println!("Received from {}: {}", addr, received);
                break;
            }
            Err(_) => {
                // Timeout, keep punching
            }
        }

        if start.elapsed().as_secs() > 30 {
            println!("Timeout after 30 seconds. Try sending from both sides simultaneously.");
            break;
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}
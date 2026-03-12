use std::net::{SocketAddr, UdpSocket, ToSocketAddrs};
use std::time::Duration;
use std::error::Error;

use sysinfo::System;
use stunclient::StunClient;

fn main() -> Result<(), Box<dyn Error>> {
    // Get kernel version
    let kernel_version = System::kernel_version().unwrap_or(String::from("Unknown Kernel Version"));
    println!("Kernel version: {}", kernel_version);

    // Bind UDP socket
    let local_socket = UdpSocket::bind("0.0.0.0:0")?;
    local_socket.set_read_timeout(Some(Duration::from_secs(30)))?;

    // Resolve STUN server hostname
    let stun_server = "stun.l.google.com:19302"
        .to_socket_addrs()?
        .next()
        .ok_or("Failed to resolve STUN server")?;
    let client = StunClient::new(stun_server);

    // Discover public endpoint
    let mapped_addr = client.query_external_address(&local_socket)?;
    println!("Public endpoint discovered via STUN: {}", mapped_addr);

    // Hole punching: input peer address
    println!("Enter peer's public IP:Port (e.g., 203.0.113.5:54321):");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let peer_addr: SocketAddr = input.trim().parse()
        .map_err(|_| "Invalid peer address format. Use IP:Port")?;

    // Send hello message
    let message = format!("Hello from kernel {}", kernel_version);
    local_socket.send_to(message.as_bytes(), peer_addr)?;

    // Receive message
    let mut buf = [0u8; 1024];
    match local_socket.recv_from(&mut buf) {
        Ok((n, addr)) => {
            let received = String::from_utf8_lossy(&buf[..n]);
            println!("Received from {}: {}", addr, received);
        }
        Err(e) => println!("No message received (timeout or error): {}", e),
    }

    Ok(())
}
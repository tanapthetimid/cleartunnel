use std::io::{self, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use stunclient::StunClient;
use sysinfo::System;

fn main() -> io::Result<()> {
    // 1. Bind an IPv4 socket to any available local port
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    // 2. Discover our public IP:Port using Google's STUN server
    let stun_addr = "stun.l.google.com:19302"
        .to_socket_addrs()?
        .find(|x| x.is_ipv4())
        .expect("Failed to resolve STUN server");
        
    let public_addr = StunClient::new(stun_addr)
        .query_external_address(&socket)
        .expect("STUN query failed - check your internet connection");

    println!("=== YOUR CONNECTION DETAILS ===");
    println!("Share this IP:Port >> {} <<", public_addr);
    println!("===============================\n");

    // 3. Receive the peer's coordinates
    print!("Enter peer's Public IP:Port (e.g., 198.51.100.1:4567): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let peer_addr: SocketAddr = input.trim().parse().expect("Invalid IP:Port format");

    // 4. Create payload with the system's kernel version
    let kernel = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let message = format!("Hello! My Linux kernel version is {}", kernel);
    let message_bytes = message.as_bytes();

    println!("\nAttempting to hole-punch to {}...", peer_addr);
    socket.set_read_timeout(Some(Duration::from_millis(1500)))?;
    let mut buf = [0u8; 1024];

    // 5. Continuous send-and-listen loop to punch the NAT hole
    loop {
        // Shoot a packet out to punch the NAT hole
        let _ = socket.send_to(message_bytes, &peer_addr);

        // Listen for the peer's incoming packet
        match socket.recv_from(&mut buf) {
            Ok((size, src)) if src == peer_addr => {
                println!(">>> CONNECTION ESTABLISHED <<<");
                println!("Received from [{}]:", src);
                println!("{}", String::from_utf8_lossy(&buf[..size]));
                
                // Final ACK to ensure their loop closes too
                let _ = socket.send_to(message_bytes, &peer_addr); 
                break;
            }
            Ok(_) => {} // Ignore stray internet background noise
            Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                // Timeout hit. Loop again and fire another packet.
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
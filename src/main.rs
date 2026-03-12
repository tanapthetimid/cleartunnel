use std::io::{self, Write};
use std::net::{SocketAddr, UdpSocket};
use std::process::Command;
use std::time::Duration;

/// Uses a dummy connection to a public IPv6 address to force the OS
/// to reveal the preferred outbound global IPv6 address.
fn get_my_ipv6() -> String {
    // 2001:4860:4860::8888 is Google's Public IPv6 DNS
    if let Ok(socket) = UdpSocket::bind("[::]:0") {
        if socket.connect("[2001:4860:4860::8888]:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    // Fallback if no internet route is found
    "::".to_string()
}

/// Fetches the OS kernel version (equivalent to `uname -r`)
fn get_kernel_version() -> String {
    #[cfg(target_family = "unix")]
    {
        if let Ok(output) = Command::new("uname").arg("-r").output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("cmd").args(["/c", "ver"]).output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    "Unknown Kernel".to_string()
}

fn main() -> io::Result<()> {
    // 1. Ask OS for port and IPv6 IP
    let my_ipv6 = get_my_ipv6();

    // Create an IPv6 UDP socket by binding to "::" (all IPv6 interfaces) on port 0
    let socket = UdpSocket::bind("[::]:0").expect("Failed to bind UDP socket");
    let assigned_port = socket.local_addr()?.port();

    println!("=== YOUR CONNECTION DETAILS ===");
    println!("Share this IPv6: {}", my_ipv6);
    println!("Share this Port: {}", assigned_port);
    println!("===============================\n");

    // 2. Receive input for peer port and IPv6 IP
    print!("Enter peer's IPv6 address: ");
    io::stdout().flush()?; // Ensure the prompt prints before waiting for input
    let mut peer_ip_str = String::new();
    io::stdin().read_line(&mut peer_ip_str)?;
    let peer_ip = peer_ip_str.trim();

    print!("Enter peer's port: ");
    io::stdout().flush()?;
    let mut peer_port_str = String::new();
    io::stdin().read_line(&mut peer_port_str)?;
    let peer_port: u16 = peer_port_str.trim().parse().expect("Invalid port number");

    // Format the peer's address as [IPv6]:Port for Rust's socket parser
    let peer_addr: SocketAddr = format!("[{}]:{}", peer_ip, peer_port)
        .parse()
        .expect("Invalid peer IPv6 address format");

    // 3. Generate the "Hello World" message with OS kernel version
    let kernel_version = get_kernel_version();
    let message = format!("Hello World! My OS kernel version is {}", kernel_version);
    let message_bytes = message.as_bytes();

    println!("\nAttempting to hole-punch to {}...", peer_addr);
    println!("Sending UDP packets... Waiting for response.\n");

    // Set a timeout so we don't get stuck waiting forever.
    // This allows the loop to trigger again if the firewall blocks the first packets.
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;

    let mut buf =[0u8; 1024];

    loop {
        // Send our message to punch the hole
        socket.send_to(message_bytes, &peer_addr)?;

        // Try to receive the peer's message
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                // 4. Print the received message
                let received_data = String::from_utf8_lossy(&buf[..size]);
                println!("\n>>> CONNECTION ESTABLISHED <<<");
                println!("Received from [{}]", src);
                println!("Message: {}", received_data);

                // Send one final time to guarantee the peer receives our message
                // before we drop the socket and exit.
                let _ = socket.send_to(message_bytes, &peer_addr);
                break;
            }
            Err(e) => {
                // If the error is a timeout (WouldBlock), it means the peer hasn't
                // punched their hole yet. We catch it and loop back to try again.
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut {
                    continue;
                } else {
                    eprintln!("An error occurred: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
use std::net::{SocketAddrV4, Ipv4Addr, TcpListener};

use errors::*;

pub fn is_port_available(address: &str, port: u16) -> bool {
    TcpListener::bind((address, port)).is_ok()
}

pub fn get_free_port() -> Result<u16> {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    // Assigning port 0 requests the OS to assign a free port
    let socket = SocketAddrV4::new(loopback, 0);
    let listener = TcpListener::bind(socket).chain_err(
        || "Failed to bind to socket even though port should be free...",
    )?;
    let port = listener.local_addr().chain_err(
        || "Failed to get port from listener",
    )?;

    Ok(port.port())
}

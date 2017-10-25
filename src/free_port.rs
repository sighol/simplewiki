use std::net::{SocketAddrV4, Ipv4Addr, TcpListener};
use std::io;

pub fn get_free_port() -> io::Result<u16> {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    // Assigning port 0 requests the OS to assign a free port
    let socket = SocketAddrV4::new(loopback, 0);
    let listener = TcpListener::bind(socket)?;
    let port = listener.local_addr()?;

    Ok(port.port())
}
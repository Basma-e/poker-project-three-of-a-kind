//! Network communication utilities for the client

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

/// Client connection wrapper for managing server communication
pub struct ClientConnection {
    pub stream: TcpStream,
    pub reader: BufReader<TcpStream>,
}

impl ClientConnection {
    /// Create a new client connection
    pub fn new(stream: TcpStream) -> Self {
        let reader = BufReader::new(stream.try_clone().unwrap());
        Self { stream, reader }
    }

    /// Send a command to the server
    pub fn send_command(&mut self, command: &str) {
        if let Err(e) = writeln!(self.stream, "{}", command) {
            eprintln!("Error sending command: {}", e);
        }
    }

    /// Read a response from the server
    pub fn read_response(&mut self) -> String {
        let mut line = String::new();
        if self.reader.read_line(&mut line).is_ok() {
            line.trim().to_string()
        } else {
            "ERROR Failed to read response".to_string()
        }
    }
}

/// Connect to the server at the given address
pub fn connect_to_server(address: &str) -> Result<ClientConnection, String> {
    match TcpStream::connect(address) {
        Ok(stream) => {
            println!("Connected to server!\n");
            let mut conn = ClientConnection::new(stream);

            // Read welcome message
            let welcome = conn.read_response();
            println!("{}", welcome);
            println!();

            Ok(conn)
        }
        Err(e) => Err(format!(
            "Failed to connect to server: {}\nMake sure the server is running on {}",
            e, address
        )),
    }
}

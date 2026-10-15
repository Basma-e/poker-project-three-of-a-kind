//! Network communication utilities for the server

use std::io::Write;
use std::net::TcpStream;

/// Send a response message to a client
pub fn send_response(stream: &mut TcpStream, message: &str) {
    if let Err(e) = writeln!(stream, "{}", message) {
        eprintln!("Error sending response: {}", e);
    }
}

/// Send multiple lines and terminate with END
#[allow(dead_code)]
pub fn send_response_lines(stream: &mut TcpStream, lines: &[String]) {
    for line in lines {
        send_response(stream, line);
    }
    send_response(stream, "END");
}

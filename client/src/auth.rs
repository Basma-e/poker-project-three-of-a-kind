//! Authentication module for client
//! Handles login and signup flows

use crate::input::{get_input, get_input_or_back};
use crate::network::ClientConnection;

/// PIN must be digits only and at least 4 digits long.
/// Returns parsed i32 if valid.
fn parse_pin_4plus(s: &str) -> Option<i32> {
    let t = s.trim();
    if t.len() < 4 { return None; }
    if !t.chars().all(|c| c.is_ascii_digit()) { return None; }
    t.parse::<i32>().ok()
}

/// Username must be non-empty and contain no whitespace.
fn parse_username(s: &str) -> Option<String> {
    let u = s.trim();
    if u.is_empty() { return None; }
    if u.chars().any(|c| c.is_whitespace()) { return None; }
    Some(u.to_lowercase())
}


/// Authenticate the user (login or signup)
/// Returns true if authentication was successful
pub fn authenticate(conn: &mut ClientConnection) -> bool {
    let mut logged_in = false;
    
    while !logged_in {
        let choice = get_input("Sign up [s] or Login [l] or Quit [q]? ", |s| {
            let s: String = s.trim().to_lowercase();
            if s == "s" || s == "signup" || s == "sign" {
                Some("signup".to_string())
            } else if s == "l" || s == "login" {
                Some("login".to_string())
            } else if s == "q" || s == "quit" || s == "exit"  {
                Some("quit".to_string())
            }
            else {
                None
            }
        }).unwrap();

        if choice == "quit" {
            return false; // authentication not successful; caller can exit program
        }

        if choice == "signup" {
            logged_in = handle_signup(conn);
        } else {
            logged_in = handle_login(conn);
        }
        
        if !logged_in {
            println!();
        }
    }
    
    logged_in
}

/// Handle user signup (type "back" or "b" at any prompt to return to main auth menu)
fn handle_signup(conn: &mut ClientConnection) -> bool {
    let username = match get_input_or_back("Enter username (no spaces, or 'back' to return): ", parse_username).unwrap() {
        Some(u) => u,
        None => return false,
    };
    let pin = match get_input_or_back("Enter PIN (at least 4 digits and greater than 1000, or 'back' to return): ", parse_pin_4plus).unwrap() {
        Some(p) => p,
        None => return false,
    };
    let balance = match get_input_or_back("Enter starting balance (or 'back' to return): ", |s| {
        s.trim().parse::<i32>().ok().filter(|&b| b > 0)
    }).unwrap() {
        Some(b) => b,
        None => return false,
    };

    // SIGNUP <username> <pin> <balance>
    let command = format!("SIGNUP {} {} {}", username, pin, balance);
    conn.send_command(&command);
    let response = conn.read_response();
    println!("{}", response);
    
    response.starts_with("OK")
}

/// Handle user login (type "back" or "b" at any prompt to return to main auth menu)
fn handle_login(conn: &mut ClientConnection) -> bool {
    let username = match get_input_or_back("Enter username (no spaces, or 'back' to return): ", parse_username).unwrap() {
        Some(u) => u,
        None => return false, // user chose back
    };
    let pin = match get_input_or_back("Enter PIN (at least 4 digits, or 'back' to return): ", parse_pin_4plus).unwrap() {
        Some(p) => p,
        None => return false,
    };
 
    // LOGIN <username> <pin>
    let command = format!("LOGIN {} {}", username, pin);
    conn.send_command(&command);

    let response = conn.read_response();
    println!("{}", response);
    
    response.starts_with("OK")
}

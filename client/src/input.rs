//! User input utilities

use std::io::{self, Write};

/// Get user input with validation
/// 
/// # Arguments
/// * `prompt` - The prompt to display to the user
/// * `validator` - A function that validates and converts the input
/// 
/// # Returns
/// * `Some(T)` - If input is valid
/// * `None` - If input is invalid (will keep prompting)
pub fn get_input<T, F>(prompt: &str, validator: F) -> Option<T>
where
    F: Fn(&str) -> Option<T>,
{
    loop {
        print!("{}", prompt);
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                if let Some(result) = validator(&input) {
                    return Some(result);
                } else {
                    println!("Invalid input. Please try again.");
                }
            }
            Err(_) => {
                println!("Error reading input. Please try again.");
            }
        }
    }
}

/// Returns true if the input is a "go back" request (e.g. from login/signup flow).
fn is_back(s: &str) -> bool {
    let t = s.trim().to_lowercase();
    t == "back" || t == "b"
}

/// Get user input with validation; user can type "back" or "b" to cancel.
/// Returns `None` if user chose back, `Some(T)` if input was valid.
pub fn get_input_or_back<T, F>(prompt: &str, validator: F) -> Option<Option<T>>
where
    F: Fn(&str) -> Option<T>,
{
    loop {
        print!("{}", prompt);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                if is_back(&input) {
                    return Some(None); // back chosen
                }
                if let Some(result) = validator(&input) {
                    return Some(Some(result));
                }
                println!("Invalid input. Please try again (or type 'back' to return).");
            }
            Err(_) => println!("Error reading input. Please try again."),
        }
    }
}

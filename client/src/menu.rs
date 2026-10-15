//! Main menu module for client
//! Handles the main menu display and routing

use crate::account;
use crate::gameplay;
use crate::input::get_input;
use crate::network::ClientConnection;

/// Run the main menu loop
pub fn run_main_loop(conn: &mut ClientConnection) {
    loop {
        show_main_menu();
        
        let choice = get_input("\nEnter choice: ", |s| {
            s.trim().parse::<u32>().ok().filter(|&n| n >= 1 && n <= 5)
        }).unwrap();

        match choice {
            1 => gameplay::play_game(conn),
            2 => account::view_stats(conn),
            3 => account::view_history(conn),
            4 => account::handle_deposit_withdraw(conn),
            5 => {
                conn.send_command("QUIT");
                let response = conn.read_response();
                println!("{}", response);
                break;
            }
            _ => {}
        }
        println!();
    }
}

/// Display the main menu
fn show_main_menu() {
    println!("=== Main Menu ===");
    println!("[1] Play Game");
    println!("[2] View Play Stats");
    println!("[3] View Balance History");
    println!("[4] Deposit/Withdraw");
    println!("[5] Quit");
}

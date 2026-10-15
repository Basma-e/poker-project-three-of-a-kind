//! Account management module for client
//! Handles viewing stats, history, and depositing funds

use crate::input::get_input;
use crate::network::ClientConnection;

/// View player statistics
pub fn view_stats(conn: &mut ClientConnection) {
    conn.send_command("VIEW_STATS");
    let response = conn.read_response();
    if response.starts_with("OK STATS") {
        println!("\n{}", response.strip_prefix("OK STATS ").unwrap_or(""));
    } else {
        println!("{}", response);
    }
}

/// View player history
pub fn view_history(conn: &mut ClientConnection) {
    conn.send_command("VIEW_HISTORY");
    let response = conn.read_response();

    if response.starts_with("OK HISTORY") {
        let payload = response.strip_prefix("OK HISTORY ").unwrap_or("");

        if payload == "No history available" {
            println!("\n{}", payload);
            return;
        }

        println!();
        for line in payload.split(" || ") {
            println!("{}", line);
        }
    } else {
        println!("{}", response);
    }
}

/// Add balance to player account or Withdraw
pub fn handle_deposit_withdraw(conn: &mut ClientConnection) {
    // Ask server for current balance first 
    conn.send_command("BALANCE");
    let bal_resp = conn.read_response();

    // Make balance display clean
    if bal_resp.starts_with("OK BALANCE") {
        println!("\nCurrent {}", bal_resp.strip_prefix("OK ").unwrap_or(&bal_resp));
    } else {
        println!("{}", bal_resp);
    }


    let action = get_input("Deposit [d], Withdraw [w], or Cancel [c]? ", |s| {
        match s.trim().to_lowercase().as_str() {
            "d" | "deposit" => Some("deposit".to_string()),
            "w" | "withdraw" => Some("withdraw".to_string()),
            "c" | "cancel" => Some("cancel".to_string()),
            _ => None,
        }
    }).unwrap();

    if action == "cancel" {
        println!("Cancelled.");
        return;
    }

    let amount = get_input("Enter amount: ", |s| {
        s.trim().parse::<i32>().ok().filter(|&x| x > 0)
    }).unwrap();

    // Send command
    let cmd = if action == "deposit" {
        format!("DEPOSIT {}", amount)
    } else {
        format!("WITHDRAW {}", amount)
    };

    conn.send_command(&cmd);
    let resp = conn.read_response();
    println!("{}", resp); // should include before/after OR at least new balance
}

# 🃏 Poker Game – Phase 1 (Five-Card Draw)

## Team Members
- **Sara Bader**
- **Basma Ebeid**
- **Labeeba Sayed**

---

## Project Overview
This project implements a **multiplayer Five-Card Draw Poker game** using a **client–server architecture** in Rust.  
The server manages all game logic, persistence, and synchronization, while multiple clients connect over TCP to play simultaneously.

Key features include:
- Account creation and login
- Persistent balances and play history using SQLite
- Betting rounds with configurable house rules
- Draw phase and automatic showdown
- Support for 2–5 players per table

---
## How to Run:
Detailed, step-by-step instructions for running and testing the game are provided in **how_to_run.md**

In summary:

1. Start the poker server
2. Launch one or more clients in separate terminals
3. Log in or sign up and follow the menu prompts to play

Please refer to how_to_run.md for:

- Server and client startup commands
- Example multi-player test scenarios
- Game flow and command usage
- Troubleshooting tips

---

## Crates & Technologies Used

### Core
- **Rust**
- **Cargo workspaces**

### Networking
- `std::net::TcpListener`
- `std::net::TcpStream`

### Persistence
- **SQLite** via `rusqlite`
- `serde` / `serde_json` for serialization

### Utilities
- `chrono` for timestamps
- `uuid` for unique player and table IDs

---

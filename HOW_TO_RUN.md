# How to Run and Test the Poker Game

## Prerequisites
- Make sure you've built the project: `cargo build --workspace`

## Step 1: Start the Server

Open a terminal and run:
```bash
cargo run -p poker
```

You should see:
```
Poker Server listening on 127.0.0.1:8080
Waiting for clients to connect...
```

**Keep this terminal open** - the server must stay running.

## Step 2: Start Client(s)

Open **separate terminal windows** for each player (you can test with 2-5 players).

For each client, run:
```bash
cargo run --package poker-gui
```

## Step 3: Test the Game Flow

### For Each Player:

1. **Login or Sign Up**
   - Choose `s` to sign up or `l` to login
   - If signing up: Enter name, PIN, and starting balance
   - If logging in: Enter your PIN

2. **Main Menu**
   - `[1]` - Play Game
   - `[2]` - View Stats
   - `[3]` - View History
   - `[4]` - Add Balance
   - `[5]` - Quit

3. **Playing a Game**
   - Select `[1]` to play
   - The first player to join will see "Waiting for more players..."
   - When 2-5 players have joined, any player can start the game
   - Follow the prompts:
     - **Betting Round**: Enter `bet <amount>`, `call`, `raise <amount>`, or `fold`
     - **Draw Phase**: Enter card positions to replace (0-4, space-separated) or `none` to stand pat
     - **Second Betting Round**: Same betting actions
     - **Showdown**: Winner is determined automatically

## Example Test Scenario

### Terminal 1 (Server):
```bash
cargo run --package poker --bin poker-server
```

### Terminal 2 (Player 1):
```bash
cargo run --package poker-gui
# Sign up: name="Alice", PIN=1234, balance=500
# Select [1] to play
# Join table, start game when ready
```

### Terminal 3 (Player 2):
```bash
cargo run --package poker-gui
# Sign up: name="Bob", PIN=5678, balance=500
# Select [1] to play
# Join table
```

### Terminal 4 (Player 3 - Optional):
```bash
cargo run --package poker-gui
# Sign up: name="Charlie", PIN=9012, balance=500
# Select [1] to play
# Join table
```

## Quick Test Commands

Once in the game:

**Betting:**
- `call` - Call the current bet
- `bet 10` - Place a bet of 10
- `raise 20` - Raise by 20
- `fold` - Fold your hand

**Draw Phase:**
- `none` - Stand pat (keep all cards)
- `0 1 2` - Replace cards at positions 0, 1, and 2
- `3` - Replace only card at position 3

## Troubleshooting

- **"Failed to connect to server"**: Make sure the server is running first
- **"Not your turn"**: Wait for your turn in the betting round
- **"Not enough players"**: Need at least 2 players to start
- **"Game already in progress"**: Wait for current hand to finish

## Notes

- Each player only sees their own hand (privacy)
- Players can view stats and history from the main menu
- Balance is persisted between sessions
- The server manages all game state

use crate::network::{ClientConnection, connect_to_server};
use eframe::egui;
use image::ImageReader;
use std::time::{Duration, Instant}; 
use egui::{FontData, FontDefinitions, FontFamily};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Auth,
    MainMenu,
    Stats,
    History,
    PlayVariantSelect,
    Playing,
    Viewer,
}

pub struct PokerUiApp {
    conn: Option<ClientConnection>,
    screen: Screen,
    status: String,
    auth_mode_signup: bool,
    username: String,
    pin: String,
    starting_balance: String,
    selected_variant: String,
    table_info: String,
    game_state: String,
    hand: String,
    stats: String,
    stats_history: String,
    history: String,
    balance_response: String,
    current_balance: String,
    amount_input: String,
    deposit_mode: bool,
    action_amount_input: String,
    draw_positions_input: String,
    last_refresh: Instant,
    last_balance_refresh: Instant,
    background: Option<egui::TextureHandle>,
    joined_table: bool,
    is_viewer: bool,
    last_results: String,
    card_textures: std::collections::HashMap<String, egui::TextureHandle>,
}

impl PokerUiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            conn: None,
            screen: Screen::Auth,
            status: String::new(),
            auth_mode_signup: false,
            username: String::new(),
            pin: String::new(),
            starting_balance: "1000".to_string(),
            selected_variant: "DRAW".to_string(),
            table_info: String::new(),
            game_state: String::new(),
            hand: String::new(),
            stats: String::new(),
            stats_history: String::new(),
            history: String::new(),
            balance_response: String::new(),
            current_balance: "Balance: --".to_string(),
            amount_input: "100".to_string(),
            deposit_mode: true,
            action_amount_input: "50".to_string(),
            draw_positions_input: "none".to_string(),
            last_refresh: Instant::now(),
            last_balance_refresh: Instant::now(),
            background: None,
            joined_table: false,
            is_viewer: false,
            last_results: String::new(),
            card_textures: std::collections::HashMap::new(),
        };

        app.connect();
        app.load_background(&cc.egui_ctx);

        let mut fonts = FontDefinitions::default();

        fonts.font_data.insert(
            "cinzel_bold".to_owned(),
            FontData::from_static(include_bytes!("../../other/Cinzel-Bold.ttf")).into(),
        );

        fonts.families.insert(
            FontFamily::Name("cinzel".into()),
            vec![
                "cinzel_bold".to_owned(),
                "Hack".to_owned(),
                "Ubuntu-Light".to_owned(),
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );

        cc.egui_ctx.set_fonts(fonts);


        app
    }

    fn connect(&mut self) {
        self.conn = connect_to_server("127.0.0.1:8080").ok();
        self.status = if self.conn.is_some() {
            "Connected to server.".to_string()
        } else {
            "Could not connect to server at 127.0.0.1:8080".to_string()
        };
    }

    fn load_background(&mut self, ctx: &egui::Context) {
        let candidates = [
            "img/Three-of-a-Kind Table.png",
            "../img/Three-of-a-Kind Table.png",
            "../../img/Three-of-a-Kind Table.png",
            // "img/Three-of-a-Kind Casino.png",
            // "../img/Three-of-a-Kind Casino.png",
            // "../../img/Three-of-a-Kind Casino.png",
        ];

        for rel in candidates {
            let path = std::path::Path::new(rel);
            if let Ok(reader) = ImageReader::open(path) {
                if let Ok(decoded) = reader.decode() {
                    let rgba = decoded.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    self.background = Some(ctx.load_texture(
                        "casino-bg",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                    self.status = format!("Loaded background image from {}", rel);
                    return;
                }
            }
        }

        self.status = "Background image not found. Tried img/, ../img/, ../../img/".to_string();
    }

    fn send(&mut self, cmd: &str) -> String {
        if let Some(conn) = self.conn.as_mut() {
            conn.send_command(cmd);
            conn.read_response()
        } else {
            "ERROR Not connected".to_string()
        }
    }

    fn extract_bool(source: &str, label: &str) -> bool {
        Self::extract_field(source, label)
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    fn refresh_account_views(&mut self) {
        self.stats = self.send("VIEW_STATS");
        self.stats_history = self.send("VIEW_HISTORY");
        self.history = self.stats_history.clone();

        let balance_resp = self.send("BALANCE");
        if balance_resp.starts_with("OK BALANCE") {
            self.current_balance = balance_resp.replacen("OK ", "", 1);
        }
    }

    fn extract_i32(source: &str, label: &str) -> i32 {
        Self::extract_field(source, label)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
    }

    fn action_flags(game_state: &str) -> (bool, bool, bool, bool, bool) {
        let my_turn = Self::extract_bool(game_state, "Your turn: ");
        let bet_to_call = Self::extract_i32(game_state, "Bet to call: ");

        if !my_turn {
            return (false, false, false, false, false);
        }

        let can_check = bet_to_call == 0;
        let can_bet = bet_to_call == 0;
        let can_call = bet_to_call > 0;
        let can_raise = bet_to_call > 0;
        let can_fold = true;

        (can_bet, can_call, can_raise, can_check, can_fold)
    }

    fn is_waiting_phase(game_state: &str) -> bool {
        game_state.contains("Phase: WaitingForPlayers")
    }

    fn is_showdown_phase(game_state: &str) -> bool {
        game_state.contains("Phase: Showdown")
    }

    fn draw_background(&self, ctx: &egui::Context) {
        if let Some(bg) = &self.background {
            let rect = ctx.content_rect();
            let painter = ctx.layer_painter(egui::LayerId::background());
            painter.image(
                bg.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::from_white_alpha(220),
            );
        }
    }

    fn refresh_game_panels(&mut self) {
        if self.last_refresh.elapsed() < Duration::from_millis(1200) {
            return;
        }
        self.last_refresh = Instant::now();

        match self.screen {
            Screen::Playing => {
                self.table_info = self.send("GET_TABLE_INFO");
                self.game_state = self.send("GET_GAME_STATE");
                self.hand = self.send("GET_HAND");
            }
            Screen::Viewer => {
                self.table_info = self.send("GET_PUBLIC_TABLE_INFO");
                self.game_state = self.send("GET_PUBLIC_GAME_STATE");
                let public_results = self.send("GET_PUBLIC_RESULTS");
                self.last_results = Self::parse_winner_block(&public_results);
            }
            _ => {}
        }
    }

    fn extract_public_board(game_state: &str) -> String {
        Self::extract_field(game_state, "Public board: ")
            .unwrap_or("N/A")
            .to_string()
    }

    fn refresh_balance(&mut self) {
        if self.last_balance_refresh.elapsed() < Duration::from_millis(2000) {
            return;
        }
        self.last_balance_refresh = Instant::now();
        let response = self.send("BALANCE");
        if response.starts_with("OK BALANCE") {
            self.current_balance = response.replacen("OK ", "", 1);
        } else if !response.starts_with("ERROR Not logged in") {
            self.current_balance = "Balance: --".to_string();
        }
    }

    fn parse_winner_block(input: &str) -> String {
        if !input.starts_with("OK SHOWDOWN") {
            return input.to_string();
        }
        let payload = input.strip_prefix("OK SHOWDOWN ").unwrap_or(input);
        let mut winner = "Unknown";
        let mut hand_type = "Unknown";
        if let Some(winners) = payload.split(" | Winners: ").nth(1) {
            winner = winners.split(" | ").next().unwrap_or("Unknown");
        }
        if let Some(all_hands) = payload
            .split("All hands: ")
            .nth(1)
            .and_then(|s| s.split(" | Winners: ").next())
        {
            let first = all_hands.split(" | ").next().unwrap_or("");
            if let Some((_name, rhs)) = first.split_once(": ") {
                if let Some((_cards, rank)) = rhs.rsplit_once(" - ") {
                    hand_type = rank;
                }
            }
        }
        format!(
            "Winning player: {}\nWinning hand type: {}\n{}",
            winner, hand_type, payload
        )
    }

    fn extract_field<'a>(source: &'a str, label: &str) -> Option<&'a str> {
        source
            .split(" | ")
            .find_map(|part| part.strip_prefix(label).map(str::trim))
    }

    /// Parses `OK STATS Rounds: n | Folds: n | Pots Won: n` into key/value pairs.
    fn parse_stats_kv(raw: &str) -> Vec<(String, String)> {
        let Some(rest) = raw.strip_prefix("OK STATS ") else {
            return vec![];
        };
        rest.split(" | ")
            .filter_map(|part| {
                let (k, v) = part.split_once(": ")?;
                Some((k.trim().to_string(), v.trim().to_string()))
            })
            .collect()
    }

    /// Splits `OK HISTORY` payload into one line per event (server uses ` || `).
    fn parse_history_entries(raw: &str) -> Result<Vec<String>, String> {
        if raw.starts_with("ERROR") {
            return Err(raw.to_string());
        }
        let Some(rest) = raw.strip_prefix("OK HISTORY ") else {
            return Err("Unexpected response format".to_string());
        };
        if rest == "No history available" {
            return Ok(vec![]);
        }
        Ok(rest
            .split(" || ")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    fn show_auth(&mut self, ui: &mut egui::Ui) {
        ui.heading("Three-of-a-Kind Casino");
        ui.label("Login or Signup");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.auth_mode_signup, false, "Login");
            ui.selectable_value(&mut self.auth_mode_signup, true, "Signup");
        });
        ui.separator();
        // ui.label("Username");
        // ui.text_edit_singleline(&mut self.username);
        // ui.label("PIN");
        // ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
        // ui.add(egui::TextEdit::singleline(&mut self.pin).password(true));
        ui.label("Username");
        ui.scope(|ui| {
            ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
            ui.text_edit_singleline(&mut self.username);
        });

        ui.label("PIN");
        ui.scope(|ui| {
            ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
            ui.add(egui::TextEdit::singleline(&mut self.pin).password(true));
        });

        
        if self.auth_mode_signup {
            ui.label("Starting balance");
            ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                ui.text_edit_singleline(&mut self.starting_balance);
            });
        }
        ui.horizontal(|ui| {
            if ui.button("Submit").clicked() {
                let response = if self.auth_mode_signup {
                    let cmd = format!(
                        "SIGNUP {} {} {}",
                        self.username.trim(),
                        self.pin.trim(),
                        self.starting_balance.trim()
                    );
                    self.send(&cmd)
                } else {
                    let cmd = format!("LOGIN {} {}", self.username.trim(), self.pin.trim());
                    self.send(&cmd)
                };
                self.status = response.clone();
                if response.starts_with("OK") {
                    self.refresh_account_views();
                    self.screen = Screen::MainMenu;
                }
            }
            if ui.button("Reconnect").clicked() {
                self.connect();
            }
        });
    }

    fn show_main_menu(&mut self, ui: &mut egui::Ui) {
        ui.heading("Main Menu");
        ui.label("Choose an action");
        ui.separator();
        if ui.button("Play game").clicked() {
                self.selected_variant = "DRAW".to_string();
                self.status.clear();
                self.screen = Screen::PlayVariantSelect;
        }

        if ui.button("Watch as viewer").clicked() {
            let resp = self.send("WATCH_GAME");
            self.status = resp.clone();

            if resp.starts_with("OK WATCHING") {
                self.is_viewer = true;
                self.joined_table = false;
                self.last_refresh = Instant::now() - Duration::from_secs(10);
                self.table_info = self.send("GET_PUBLIC_TABLE_INFO");
                self.game_state = self.send("GET_PUBLIC_GAME_STATE");

                let public_results = self.send("GET_PUBLIC_RESULTS");
                self.last_results = Self::parse_winner_block(&public_results);
                self.screen = Screen::Viewer;
            }
        }
        if ui.button("View stats").clicked() {
            self.stats = self.send("VIEW_STATS");
            self.screen = Screen::Stats;
        }
        if ui.button("View balance history").clicked() {
            self.history = self.send("VIEW_HISTORY");
            self.screen = Screen::History;
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.deposit_mode, true, "Deposit");
            ui.selectable_value(&mut self.deposit_mode, false, "Withdraw");

            ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                ui.add(egui::TextEdit::singleline(&mut self.amount_input).desired_width(80.0));
            });

            if ui.button("Submit").clicked() {
                let cmd = if self.deposit_mode {
                    format!("DEPOSIT {}", self.amount_input.trim())
                } else {
                    format!("WITHDRAW {}", self.amount_input.trim())
                };
                self.balance_response = self.send(&cmd);
                self.refresh_account_views();
            }
        });
        if ui.button("Sign out").clicked() {
            self.send("QUIT");
            self.conn = None;
            self.connect();
            self.screen = Screen::Auth;
        }
        if !self.balance_response.is_empty() {
            ui.separator();
            ui.label(self.balance_response.clone());
        }
    }

    fn show_stats_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Player Stats");
        ui.separator();
        if self.stats.is_empty() {
            self.stats = self.send("VIEW_STATS");
        }
        if self.stats_history.is_empty() {
            self.stats_history = self.send("VIEW_HISTORY");
        }

        if self.stats.starts_with("ERROR") {
            ui.label(
                egui::RichText::new(&self.stats)
                    .color(egui::Color32::from_rgb(160, 40, 40))
                    .strong(),
            );
        } else {
            let pairs = Self::parse_stats_kv(&self.stats);
            egui::Frame::default()
                .fill(egui::Color32::from_white_alpha(200))
                .corner_radius(8.0)
                .inner_margin(20.0)
                .show(ui, |ui| {
                    if pairs.is_empty() {
                        ui.label(egui::RichText::new(&self.stats).monospace());
                    } else {
                        egui::Grid::new("stats_grid")
                            .num_columns(2)
                            .spacing([24.0, 14.0])
                            .show(ui, |ui| {
                                for (label, value) in &pairs {
                                    ui.label(
                                        egui::RichText::new(label).strong().size(22.0),
                                    );
                                    ui.label(
                                        egui::RichText::new(value)
                                            .strong()
                                            .size(28.0)
                                            .color(egui::Color32::BLACK),
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
        }

        ui.separator();
        ui.heading("Recent Hand Results");
        ui.separator();

        let recent_entries = match Self::parse_history_entries(&self.stats_history) {
            Err(msg) => {
                ui.label(
                    egui::RichText::new(msg.as_str())
                        .color(egui::Color32::from_rgb(160, 40, 40))
                        .strong(),
                );
                vec![]
            }
            Ok(lines) => lines
                .into_iter()
                .filter(|l| l.to_lowercase().contains("hand won"))
                .collect::<Vec<_>>(),
        };

        if recent_entries.is_empty() {
            ui.label(
                egui::RichText::new("No hand results yet.")
                    .italics()
                    .size(22.0),
            );
        } else {
            egui::Frame::default()
                .fill(egui::Color32::from_white_alpha(200))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    egui::Grid::new("recent_hands_grid")
                        .num_columns(3)
                        .spacing([24.0, 12.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Date").strong().size(22.0));
                            ui.label(egui::RichText::new("Game Type").strong().size(22.0));
                            ui.label(egui::RichText::new("Pot Won").strong().size(22.0));
                            ui.end_row();

                            for line in recent_entries.iter().rev().take(5) {
                                let date = line
                                    .find('[')
                                    .and_then(|s| line[s + 1..].find(']').map(|e| s + 1 + e))
                                    .and_then(|end| {
                                        let start = line.find('[')? + 1;
                                        let ts = &line[start..end];
                                        let date_part = ts.split('T').next().unwrap_or(ts);
                                        Some(date_part.split_whitespace().next().unwrap_or(date_part).to_string())
                                    })
                                    .unwrap_or_else(|| "Unknown".to_string());

                                let game_type = line
                                    .split('(')
                                    .nth(1)
                                    .and_then(|s| s.split(')').next())
                                    .unwrap_or("Unknown");

                                let pot_won = line
                                    .split("Hand won: +")
                                    .nth(1)
                                    .and_then(|s| s.split_whitespace().next())
                                    .unwrap_or("?");

                                ui.label(
                                    egui::RichText::new(date).strong().size(22.0),
                                );
                                ui.label(
                                    egui::RichText::new(game_type)
                                        .strong()
                                        .size(22.0),
                                );
                                ui.label(
                                    egui::RichText::new(format!("+{}", pot_won))
                                        .strong()
                                        .size(22.0)
                                        .color(egui::Color32::BLACK),
                                );
                                ui.end_row();
                            }
                        });
                });
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                self.stats = self.send("VIEW_STATS");
                self.stats_history = self.send("VIEW_HISTORY");
            }
            if ui.button("Back to menu").clicked() {
                self.screen = Screen::MainMenu;
            }
        });
    }

    fn show_history_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Balance History");
        ui.separator();
        if self.history.is_empty() {
            self.history = self.send("VIEW_HISTORY");
        }

        let entries = Self::parse_history_entries(&self.history);

        egui::ScrollArea::vertical()
            .max_height(420.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match &entries {
                    Err(msg) => {
                        ui.label(
                            egui::RichText::new(msg.as_str())
                                .color(egui::Color32::from_rgb(160, 40, 40))
                                .strong(),
                        );
                    }
                    Ok(lines) if lines.is_empty() => {
                        ui.label(
                            egui::RichText::new("No transactions or hand results yet.")
                                .italics()
                                .size(22.0),
                        );
                    }
                    Ok(lines) => {
                        for (i, line) in lines.iter().enumerate() {
                            let bg = if i % 2 == 0 {
                                egui::Color32::from_white_alpha(180)
                            } else {
                                egui::Color32::from_white_alpha(120)
                            };
                            egui::Frame::default()
                                .fill(bg)
                                .corner_radius(6.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        let lower = line.to_lowercase();
                                        let icon = if lower.contains("deposit") {
                                            "+"
                                        } else if lower.contains("withdraw") {
                                            "−"
                                        } else if lower.contains("hand won") {
                                            "★"
                                        } else {
                                            "•"
                                        };
                                        ui.label(
                                            egui::RichText::new(icon)
                                                .strong()
                                                .size(20.0)
                                                .color(egui::Color32::BLACK),
                                        );
                                        ui.label(
                                            egui::RichText::new(line)
                                                .strong()
                                                .size(20.0)
                                                .color(egui::Color32::BLACK),
                                        );
                                    });
                                });
                            ui.add_space(6.0);
                        }
                    }
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                self.history = self.send("VIEW_HISTORY");
            }
            if ui.button("Back to menu").clicked() {
                self.screen = Screen::MainMenu;
            }
        });
    }


    fn show_variant_select(&mut self, ui: &mut egui::Ui) {
        ui.heading("Select Poker Variant");
        ui.radio_value(
            &mut self.selected_variant,
            "DRAW".to_string(),
            "Five-Card Draw",
        );
        ui.radio_value(
            &mut self.selected_variant,
            "HOLDEM".to_string(),
            "Texas Hold'em",
        );
        ui.radio_value(
            &mut self.selected_variant,
            "STUD".to_string(),
            "Seven-card Stud",
        );

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("Continue").clicked() {
                if self.selected_variant == "STUD" {
                    self.status = "Seven-card Stud is coming soon.".to_string();
                } else {
                    let join_resp = self.send("JOIN_TABLE");
                    self.status = join_resp.clone();

                    if join_resp.starts_with("OK JOIN") {
                        self.joined_table = true;
                        self.is_viewer = false;

                        let variant_resp = self.send(&format!("SET_VARIANT {}", self.selected_variant));

                        let can_continue =
                            variant_resp.starts_with("OK VARIANT")
                            || variant_resp.starts_with("ERROR Variant already locked")
                            || variant_resp.starts_with("ERROR Only dealer can choose variant");

                        self.status = variant_resp;

                        if can_continue {
                            self.last_refresh = Instant::now() - Duration::from_secs(10);
                            self.table_info = self.send("GET_TABLE_INFO");
                            self.game_state = self.send("GET_GAME_STATE");
                            self.hand = self.send("GET_HAND");
                            self.screen = Screen::Playing;
                        }
                    }
                }
            }
            if ui.button("Back to menu").clicked() {
                self.send("LEAVE_TABLE");
                self.screen = Screen::MainMenu;
            }
        });
    }


    //instead of drawing use omages:
    fn rank_to_file(rank: &str) -> Option<&'static str> {
        match rank {
            "2" => Some("2"),
            "3" => Some("3"),
            "4" => Some("4"),
            "5" => Some("5"),
            "6" => Some("6"),
            "7" => Some("7"),
            "8" => Some("8"),
            "9" => Some("9"),
            "10" => Some("10"),
            "J" => Some("J"),
            "Q" => Some("Q"),
            "K" => Some("K"),
            "A" => Some("A"),
            _ => None,
        }
    }

    //got deck online so using their suit symbols, map to our file naming:
    fn suit_to_file(suit: char) -> Option<&'static str> {
        match suit {
            '♥' => Some("C"), // C = hearts
            '♦' => Some("H"), // H = diamonds
            '♣' => Some("D"), // D = clubs
            '♠' => Some("S"),
            _ => None,
        }
    }

    fn card_token_to_filename(card: &str) -> Option<String> {
        let suit = card.chars().last()?;
        let suit_code = Self::suit_to_file(suit)?;

        let rank_part = &card[..card.len() - suit.len_utf8()];
        let rank_code = Self::rank_to_file(rank_part.trim())?;

        Some(format!("{}{}.png", suit_code, rank_code))
    }

    fn load_card_texture(
        &mut self,
        ctx: &egui::Context,
        filename: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(tex) = self.card_textures.get(filename) {
            return Some(tex.clone());
        }

        let candidates = [
            format!("img/deck/{}", filename),
            format!("../img/deck/{}", filename),
            format!("../../img/deck/{}", filename),
        ];

        for rel in candidates {
            let path = std::path::Path::new(&rel);
            if let Ok(reader) = ImageReader::open(path) {
                if let Ok(decoded) = reader.decode() {
                    let rgba = decoded.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let tex = ctx.load_texture(
                        rel.clone(),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.card_textures.insert(filename.to_string(), tex.clone());
                    return Some(tex);
                }
            }
        }

        None
    }


    //parse cards:
    fn parse_cards_from_text(raw: &str) -> Vec<String> {
        raw.split_whitespace()
            .filter(|tok| tok.chars().any(|c| "♠♥♦♣".contains(c)))
            .map(|s| s.to_string())
            .collect()
    }

    fn draw_cards_row(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, cards: &[String]) {
        ui.horizontal_wrapped(|ui| {
            for card in cards {
                if let Some(filename) = Self::card_token_to_filename(card) {
                    if let Some(tex) = self.load_card_texture(ctx, &filename) {
                        let image = egui::Image::new((tex.id(), egui::vec2(72.0, 104.0)));
                        ui.add(image);
                    } else {
                        ui.label(card);
                    }
                } else {
                    ui.label(card);
                }
            }
        });
    }

    fn show_playing(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.refresh_game_panels();

        let phase = Self::extract_field(&self.game_state, "Phase: ")
            .unwrap_or("Unknown")
            .to_string();
        let my_turn = Self::extract_bool(&self.game_state, "Your turn: ");
        let bet_to_call = Self::extract_i32(&self.game_state, "Bet to call: ");
        let variant = Self::extract_field(&self.table_info, "Variant: ")
            .or_else(|| Self::extract_field(&self.game_state, "Variant: "))
            .unwrap_or("Unknown")
            .to_string();
        let current_actor = Self::extract_field(&self.game_state, "Current actor: ")
            .unwrap_or("N/A")
            .to_string();
        let next_actor = Self::extract_field(&self.game_state, "Next: ")
            .unwrap_or("N/A")
            .to_string();
        let ante = Self::extract_field(&self.game_state, "Ante: ")
            .unwrap_or("0")
            .to_string();
        let board = Self::extract_field(&self.game_state, "Board: ")
            .unwrap_or("N/A")
            .to_string();
        let blinds = Self::extract_field(&self.game_state, "Blinds: ")
            .unwrap_or("N/A")
            .to_string();

        let (can_bet, can_call, can_raise, can_check, can_fold) =
            Self::action_flags(&self.game_state);

        ui.heading(format!("Poker Table — {}", variant));
        ui.label(format!("Phase: {}", phase));
        ui.separator();

        if Self::is_waiting_phase(&self.game_state) {
            ui.heading(format!("Lobby — {}", variant));
            ui.label("Waiting for players to join the table.");
            ui.separator();

            ui.label(self.table_info.clone());
            
            
            if !self.last_results.is_empty() {
                ui.separator();
                ui.heading("Previous Hand Results");
                ui.label(self.last_results.clone());
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Refresh lobby").clicked() {
                    self.last_refresh = Instant::now() - Duration::from_secs(10);
                    self.last_results.clear();
                    self.refresh_game_panels();
                }

                if ui.button("Start game").clicked() {
                    let resp = self.send("START_GAME");
                    self.status = resp;
                    self.last_refresh = Instant::now() - Duration::from_secs(10);
                    self.refresh_game_panels();
                }

                if ui.button("Leave table").clicked() {
                    let _ = self.send("LEAVE_TABLE");
                    self.joined_table = false;
                    self.screen = Screen::MainMenu;
                }
            });

            ui.separator();
            ui.collapsing("Status / server response", |ui| {
                ui.label(self.status.clone());
            });

            return;
        }

        ui.columns(2, |columns| {
            let (left_slice, right_slice) = columns.split_at_mut(1);
            let left = &mut left_slice[0];
            let right = &mut right_slice[0];

            left.label(format!("Current actor: {}", current_actor));
            left.label(format!("Next: {}", next_actor));
            left.label(format!("Bet to call: {}", bet_to_call));

            if variant == "TexasHoldem" {
                left.label(format!("Blinds: {}", blinds));
            } else {
                left.label(format!("Ante: {}", ante));
            }

            left.add_space(10.0);

            if phase == "BettingRound" {
                left.heading("Betting Controls");

                if my_turn {
                    left.label("It is your turn.");

                    left.horizontal(|ui| {
                        ui.label("Amount:");
                        ui.scope(|ui| {
                            ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.action_amount_input)
                                    .desired_width(90.0),
                            );
                        });
                    });

                    left.horizontal_wrapped(|ui| {
                        if ui.add_enabled(can_check, egui::Button::new("Check")).clicked() {
                            self.status = self.send("CHECK");
                            self.last_refresh = Instant::now() - Duration::from_secs(10);
                        }
                        if ui.add_enabled(can_call, egui::Button::new("Call")).clicked() {
                            self.status = self.send("CALL");
                            self.last_refresh = Instant::now() - Duration::from_secs(10);
                        }
                        if ui.add_enabled(can_fold, egui::Button::new("Fold")).clicked() {
                            self.status = self.send("FOLD");
                            self.last_refresh = Instant::now() - Duration::from_secs(10);
                        }
                        if ui.add_enabled(can_bet, egui::Button::new("Bet")).clicked() {
                            self.status =
                                self.send(&format!("BET {}", self.action_amount_input.trim()));
                            self.last_refresh = Instant::now() - Duration::from_secs(10);
                        }
                        if ui.add_enabled(can_raise, egui::Button::new("Raise")).clicked() {
                            self.status =
                                self.send(&format!("RAISE {}", self.action_amount_input.trim()));
                            self.last_refresh = Instant::now() - Duration::from_secs(10);
                        }
                    });
                } else {
                    left.label("Waiting for your turn.");
                }
            }

            if phase == "Draw" {
                left.heading("Draw Controls");

                if my_turn {
                    left.label("Enter card positions 0-4, or none.");
                    left.scope(|ui| {
                        ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.draw_positions_input)
                                .desired_width(180.0),
                        );
                    });

                    if left.button("Submit draw").clicked() {
                        let draw_cmd = if self.draw_positions_input.trim().is_empty()
                            || self.draw_positions_input.trim().eq_ignore_ascii_case("none")
                        {
                            "DRAW none".to_string()
                        } else {
                            format!("DRAW {}", self.draw_positions_input.trim())
                        };

                        self.status = self.send(&draw_cmd);
                        self.last_refresh = Instant::now() - Duration::from_secs(10);
                    }
                } else {
                    left.label("Waiting for your turn.");
                }
            }

            right.label("Your hand");
            let hand_cards = Self::parse_cards_from_text(&self.hand);
            if hand_cards.is_empty() {
                right.label(self.hand.clone());
            } else {
                self.draw_cards_row(right, ctx, &hand_cards);
            }

            if !board.is_empty() && board != "N/A" {
                right.add_space(12.0);
                right.label("Community board");
                let board_cards = Self::parse_cards_from_text(&board);
                if board_cards.is_empty() {
                    right.label(board.clone());
                } else {
                    self.draw_cards_row(right, ctx, &board_cards);
                }
            }
        });

        if Self::is_showdown_phase(&self.game_state) {
            ui.separator();
            ui.heading("Showdown");

            let showdown = self.send("SHOWDOWN");
            self.last_results = Self::parse_winner_block(&showdown);
            self.refresh_account_views();    
            ui.label(self.last_results.clone());
        }

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Refresh now").clicked() {
                self.last_refresh = Instant::now() - Duration::from_secs(10);
                self.refresh_game_panels();
            }

            if ui.button("Show round results").clicked() {
                let showdown = self.send("SHOWDOWN");
                self.last_results = Self::parse_winner_block(&showdown);
                self.status = self.last_results.clone();
                self.refresh_account_views();

            }

            if ui.button("Back to menu").clicked() {
                let _ = self.send("LEAVE_TABLE");
                self.joined_table = false;
                self.screen = Screen::MainMenu;
            }
        });

        ui.separator();
        ui.collapsing("Status / server response", |ui| {
            ui.label(self.status.clone());
        });
    }

    fn show_viewer(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.refresh_game_panels();

        let variant = Self::extract_field(&self.table_info, "Variant: ")
            .or_else(|| Self::extract_field(&self.game_state, "Variant: "))
            .unwrap_or("Unknown")
            .to_string();
        let phase = Self::extract_field(&self.game_state, "Phase: ")
            .unwrap_or("Unknown")
            .to_string();
        let board = Self::extract_public_board(&self.game_state);

        ui.heading(format!("Viewer Mode — {}", variant));
        ui.label(format!("Phase: {}", phase));
        ui.separator();

        ui.label("Public table info");
        ui.label(self.table_info.clone());
        ui.separator();

        if !board.is_empty() && board != "N/A" {
            ui.label("Community board");
            let board_cards = Self::parse_cards_from_text(&board);
            if board_cards.is_empty() {
                ui.label(board.clone());
            } else {
                self.draw_cards_row(ui, ctx, &board_cards);
            }
            ui.separator();
        }

        ui.label("Latest public result");
        ui.label(self.last_results.clone());
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Refresh now").clicked() {
                self.last_refresh = Instant::now() - Duration::from_secs(10);
                self.table_info = self.send("GET_PUBLIC_TABLE_INFO");
                self.game_state = self.send("GET_PUBLIC_GAME_STATE");
                let public_results = self.send("GET_PUBLIC_RESULTS");
                self.last_results = Self::parse_winner_block(&public_results);
            }

            if ui.button("View latest hand results").clicked() {
                let resp = self.send("GET_PUBLIC_RESULTS");
                self.last_results = Self::parse_winner_block(&resp);
                self.status = self.last_results.clone();
            }

            if ui.button("Back to menu").clicked() {
                let _ = self.send("STOP_WATCHING");
                self.is_viewer = false;
                self.screen = Screen::MainMenu;
            }
        });
    }

}

impl eframe::App for PokerUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::light());
        ctx.set_pixels_per_point(1.2);

        //font stuff:
        let mut fonts = FontDefinitions::default();

        fonts.font_data.insert(
            "cinzel_bold".to_owned(),
            FontData::from_static(include_bytes!("../../other/Cinzel-Bold.ttf")).into(),
        );

        fonts.families.insert(
            FontFamily::Name("cinzel".into()),
            vec![
                "cinzel_bold".to_owned(),
                "Hack".to_owned(),
                "Ubuntu-Light".to_owned(),
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );

        ctx.set_fonts(fonts);

        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(34.0, egui::FontFamily::Name("cinzel".into())),
            // egui::FontId::new(34.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(22.0, egui::FontFamily::Monospace),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(20.0, egui::FontFamily::Proportional),
        );
        // style.visuals.override_text_color = Some(egui::Color32::BLACK);
        // style.visuals.widgets.noninteractive.fg_stroke =
        //     egui::Stroke::new(2.0, egui::Color32::BLACK);
        // style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(2.0, egui::Color32::BLACK);
        // style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(2.0, egui::Color32::BLACK);
        // style.visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, egui::Color32::BLACK);
        // style.visuals.widgets.open.fg_stroke = egui::Stroke::new(2.0, egui::Color32::BLACK);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);

        style.visuals.selection.bg_fill = egui::Color32::from_rgb(170, 30, 30);
        style.visuals.selection.stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(110, 15, 15));

        style.visuals.override_text_color = Some(egui::Color32::WHITE);
        style.visuals.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(2.0, egui::Color32::WHITE);
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
        style.visuals.widgets.open.fg_stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
        
        //chnage button fill color:
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(170, 30, 30);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(200, 40, 40);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(140, 20, 20);
        
        style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(170, 30, 30);
        style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(200, 40, 40);
        style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(140, 20, 20);

        ctx.set_style(style);

        self.draw_background(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 45))
                    .inner_margin(16.0),
            )
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
                // ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                self.refresh_balance();
                // ui.horizontal(|ui| {
                //     ui.add_space(ui.available_width().max(0.0) - 260.0);
                //     ui.label(egui::RichText::new(self.current_balance.clone()).strong());
                // });
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width().max(0.0) - 260.0);
                    ui.label(
                        egui::RichText::new(self.current_balance.clone())
                            .strong()
                            .size(28.0),
                    );
                });
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.set_max_width(980.0);
                    match self.screen {
                        Screen::Auth => self.show_auth(ui),
                        Screen::MainMenu => self.show_main_menu(ui),
                        Screen::Stats => self.show_stats_page(ui),
                        Screen::History => self.show_history_page(ui),
                        Screen::PlayVariantSelect => self.show_variant_select(ui),
                        Screen::Playing => self.show_playing(ui, ctx),
                        Screen::Viewer => self.show_viewer(ui, ctx),
                    }
                });
            });
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}

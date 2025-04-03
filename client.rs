use egui::{Button, Color32, Frame};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::exit;
#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum GameState {
    Waiting,
    InGame,
    GameOver,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum Turn {
    MousePlayer,
    TrapperPlayer,
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]

enum RoomType {
    SinglePlayer,
    MultiPlayer,
}
#[derive(Serialize, Deserialize, Clone)]

enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Serialize, Deserialize, Clone)]
struct Room {
    room_id: u32,
    room_name: String,
    mouse_player: Option<String>,
    trapper_player: Option<String>,
    game_state: GameState,
    mouse_position: (u32, u32),
    walls: Vec<(u32, u32)>,
    turn: Turn,
    winner: Option<Turn>,
    room_type: RoomType,
    game_difficulty: Option<Difficulty>,
    mouse_player_exited: bool,
    trapper_player_exited: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Server {
    rooms: Vec<Room>,
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Trap The Mouse",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}

enum AppState {
    Login,
    Menu,
    Rooms,
    InGame,
    Lobby,
    MenuSinglePlayer,
    InGameSinglePlayer,
    GameOver,
}
struct MyApp {
    stream: Option<TcpStream>,
    username: String,
    app_state: AppState,
    new_room_name: String,
    last_update_time: std::time::Instant,
    update_interval: std::time::Duration,
    server_data: Option<Server>,
    current_room: Option<u32>,
    current_role: Option<Turn>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            stream: TcpStream::connect("127.0.0.1:8080").ok(),
            username: String::new(),
            app_state: AppState::Login,
            new_room_name: String::new(),
            last_update_time: std::time::Instant::now(),
            update_interval: std::time::Duration::from_millis(10),
            server_data: None,
            current_room: None,
            current_role: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| match self.app_state {
                AppState::Login => self.render_login(ui),
                AppState::Menu => self.render_menu(ui),
                AppState::InGame => self.render_game(ui),
                AppState::Rooms => self.render_rooms(ui),
                AppState::Lobby => self.render_lobby(ui),
                AppState::MenuSinglePlayer => self.render_menu_single_player(ui),
                AppState::GameOver => self.render_game_over(ui),
                AppState::InGameSinglePlayer => self.render_game_single_player(ui),
            });
            if self.stream.is_none()
            {
                exit(1);
            }
        });
        self.get_updates();
    }
}

impl MyApp {

    fn is_surrounded(mouse_pos: (u32, u32), traps: &[(u32, u32)]) -> bool {
        let (mouse_x, mouse_y) = mouse_pos;
        let neighbors = if mouse_x % 2 == 0 {
            vec![
                (mouse_x, mouse_y - 1),
                (mouse_x, mouse_y + 1),
                (mouse_x - 1, mouse_y),
                (mouse_x + 1, mouse_y),
                (mouse_x - 1, mouse_y - 1),
                (mouse_x + 1, mouse_y - 1),
            ]
        } else {
            vec![
                (mouse_x, mouse_y - 1),
                (mouse_x, mouse_y + 1),
                (mouse_x - 1, mouse_y),
                (mouse_x + 1, mouse_y),
                (mouse_x - 1, mouse_y + 1),
                (mouse_x + 1, mouse_y + 1),
            ]
        };

        neighbors.into_iter().all(|pos| traps.contains(&pos))
    }

    fn send_command(&mut self, message: &str) {
        if let Some(ref mut stream) = self.stream {
            if let Err(e) = stream.write_all(message.as_bytes()) {
                println!("ERR:mesaj spre server {}", e);
            }
        } else {
            println!("No connection");
        }
    }

    fn get_updates(&mut self) {
        if self.last_update_time.elapsed() >= self.update_interval {
            if let Some(ref mut stream) = self.stream {
                if let Err(e) = stream.write_all(b"get_update") {
                    println!("Server stopped the connection {}", e);
                    self.stream = None;
                    exit(1);
                }

                let mut buffer = [0; 2048];
                match stream.read(&mut buffer) {
                    Ok(0) => {
                        println!("Server stopped the stream");
                        self.stream = None;
                    }
                    Ok(n) => {
                        let message = String::from_utf8_lossy(&buffer[..n]);

                        match serde_json::from_str::<Server>(&message) {
                            Ok(server_data) => {
                                self.server_data = Some(server_data.clone());
                            }
                            Err(e) => {
                                println!("ERR:serialization {}", e);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        println!("ERR: response  {}", e);
                        self.stream = None;
                    }
                }
            }

            self.last_update_time = std::time::Instant::now();
        }
    }

    fn render_login(&mut self, ui: &mut egui::Ui) {
        ui.add_space(80.0);
        ui.heading("Trap The Mouse!");
        ui.add_space(40.0);

        ui.label("Username:");
        ui.add_space(5.0);

        ui.add(egui::TextEdit::singleline(&mut self.username));
        ui.add_space(30.0);

        if ui.button("Play").clicked() {
            if self.username.is_empty() {
                ui.label("Enter username: ");
            } else {
                let command = format!("login {} ", self.username);
                self.send_command(&command);
                self.app_state = AppState::Menu;
            }
        }
    }

    fn render_menu(&mut self, ui: &mut egui::Ui) {
        if let Some(server_data) = &self.server_data.clone() {
            ui.add_space(80.0);
            ui.heading("Trap The Mouse!");
            ui.label(format!("Connected as: {} ", self.username));

            ui.add_space(30.0);

            ui.heading("Menu");
            ui.add_space(30.0);

            if ui.button("SinglePlayer").clicked() {
                let room_id = server_data.rooms.len() + 1;
                let command = format!("create_single_room {} ", self.username);
                self.current_role = Some(Turn::TrapperPlayer);
                self.current_room = Some(room_id as u32);
                self.send_command(&command);

                self.app_state = AppState::MenuSinglePlayer;
            }
            ui.add_space(20.0);

            if ui.button("MultiPlayer").clicked() {
                self.app_state = AppState::Rooms;
            }
            ui.add_space(30.0);

            if ui.button("Back").clicked() {
                self.username.clear();
                self.app_state = AppState::Login;
            }
        }
    }

    fn render_rooms(&mut self, ui: &mut egui::Ui) {
        ui.add_space(80.0);
        ui.heading("Trap The Mouse!");
        ui.label(format!("Connected as: {} ", self.username));

        ui.add_space(30.0);

        ui.heading("Create a room");

        ui.vertical_centered(|ui| {
            ui.label("Room Name");
            ui.add(egui::TextEdit::singleline(&mut self.new_room_name));

            if ui.button("Create Room").clicked() {
                if self.new_room_name.is_empty()
                {
                    println!("insert a room name");
                }
                else {
                    let command = format!("create_room {} ", self.new_room_name);
                    self.send_command(&command);
                    self.new_room_name.clear();
                    
                }
            }
        });

        ui.heading("Rooms");
        ui.add_space(30.0);

        let rooms = if let Some(server_data) = &self.server_data {
            server_data.rooms.clone()
        } else {
            vec![]
        };

        ui.horizontal(|ui| {
            ui.add_space(250.0);

            ui.vertical_centered(|ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for room in &rooms {
                        if room.room_type == RoomType::MultiPlayer {
                            ui.horizontal(|ui| {
                                ui.label(&room.room_name);

                                let room_id = room.room_id;

                                if room.mouse_player.is_none()
                                    && ui.button("Join as Mouse").clicked()
                                {
                                    let command = format!(
                                        "join_room {} mouse {} ",
                                        room_id,
                                        self.username.clone()
                                    );
                                    println!("{}", command);

                                    self.send_command(&command);
                                    self.app_state = AppState::Lobby;
                                    self.current_room = Some(room_id);
                                    self.current_role = Some(Turn::MousePlayer);
                                }
                                if room.trapper_player.is_none() && ui.button("Join as Trapper").clicked() {
                                        let command = format!(
                                            "join_room {} trapper {} ",
                                            room_id,
                                            self.username.clone()
                                        );
                                        println!("{}", command);
                                        self.send_command(&command);
                                        self.app_state = AppState::Lobby;
                                        self.current_room = Some(room_id);
                                        self.current_role = Some(Turn::TrapperPlayer);
                                    }
                                if room.trapper_player.is_some() && room.mouse_player.is_some() {
                                    ui.label("Room is full.");
                                }
                            });
                        }
                    }
                });
                ui.add_space(10.0);
            });
        });

        if ui.button("Back to Lobby").clicked() {
            self.app_state = AppState::Menu;
        }
    }

    fn render_game(&mut self, ui: &mut egui::Ui) {
        if let Some(server_data) = &self.server_data.clone() {
            if let Some(room) = server_data
                .rooms
                .iter()
                .find(|room| Some(room.room_id) == self.current_room)
            {
                let current_role = self.current_role.clone();

                let (x, y) = room.mouse_position;
                let is_on_edge = x == 0 || x == 10 || y == 0 || y == 10;

                if is_on_edge {
                    let command = format!("game_over {} mouse ", room.room_id);
                    self.send_command(&command)
                } else if MyApp::is_surrounded(room.mouse_position, &room.walls) {
                    let command = format!("game_over {} trapper ", room.room_id);
                    self.send_command(&command)
                }
                if room.game_state == GameState::GameOver {
                    self.app_state = AppState::GameOver;
                } else {
                    let mut command_to_send: Option<String> = None;

                    ui.horizontal(|ui| {
                        ui.add_space(300.0);
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(">Mouse Player<");
                            ui.horizontal(|ui| {
                                ui.add_space(50.0);
                                ui.label(room.mouse_player.as_ref().unwrap());
                            });
                        });
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(">Trapper Player<");
                            ui.horizontal(|ui| {
                                ui.add_space(50.0);
                                ui.label(room.trapper_player.as_ref().unwrap());
                            });
                        });
                    });

                    ui.add_space(10.0);
                    ui.label(">MOVING NOW<");

                    if current_role.as_ref().unwrap() == &room.turn {
                        ui.label("YOU");
                    } else {
                        ui.label("OPPONENT");
                    }

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.add_space(250.0);
                        ui.vertical(|ui| {
                            for x in 0..11 {
                                ui.horizontal(|ui| {
                                    if x % 2 == 1 {
                                        ui.add_space(15.0);
                                    }
                                    for y in 0..11 {
                                        let base_color = if (x, y) == room.mouse_position {
                                            Color32::from_gray(200)
                                        } else if room.walls.contains(&(x, y)) {
                                            Color32::from_rgb(255, 0, 0)
                                        } else {
                                            Color32::from_gray(100)
                                        };

                                        let hover_color = Color32::from_rgb(255, 165, 0);

                                        Frame::none()
                                            .fill(base_color)
                                            .rounding(egui::Rounding::same(5.0))
                                            .show(ui, |ui| {
                                                let button_response = ui.add(
                                                    Button::new("")
                                                        .frame(false)
                                                        .rounding(egui::Rounding::same(5.0))
                                                        .min_size(egui::vec2(20.0, 20.0)),
                                                );
                                                if button_response.clicked()
                                                    && (x, y) != room.mouse_position
                                                    && !room.walls.contains(&(x, y))
                                                {
                                                    if current_role == Some(Turn::MousePlayer) {
                                                        let (mouse_x, mouse_y) =
                                                            room.mouse_position;

                                                        let is_clickable = if mouse_x % 2 == 0 {
                                                            (x == mouse_x
                                                                && (y == mouse_y - 1
                                                                    || y == mouse_y + 1))
                                                                || (x == mouse_x - 1
                                                                    && y == mouse_y)
                                                                || (x == mouse_x + 1
                                                                    && y == mouse_y)
                                                                || (x == mouse_x - 1
                                                                    && y == mouse_y - 1)
                                                                || (x == mouse_x + 1
                                                                    && y == mouse_y - 1)
                                                        } else {
                                                            (x == mouse_x
                                                                && (y == mouse_y - 1
                                                                    || y == mouse_y + 1))
                                                                || (x == mouse_x - 1
                                                                    && y == mouse_y)
                                                                || (x == mouse_x + 1
                                                                    && y == mouse_y)
                                                                || (x == mouse_x - 1
                                                                    && y == mouse_y + 1)
                                                                || (x == mouse_x + 1
                                                                    && y == mouse_y + 1)
                                                        };

                                                        if is_clickable
                                                            && room.turn == Turn::MousePlayer
                                                        {
                                                            command_to_send = Some(format!(
                                                                "move_mouse {} {} {} ",
                                                                room.room_id, x, y
                                                            ));
                                                        }
                                                    } else if room.turn == Turn::TrapperPlayer {
                                                            command_to_send = Some(format!(
                                                                    "place_trap {} {} {} ",
                                                                    room.room_id, x, y
                                                                ));
                                                            }
                                                    
                                                } else if button_response.hovered()
                                                    && (x, y) != room.mouse_position
                                                    && !room.walls.contains(&(x, y))
                                                {
                                                    if current_role == Some(Turn::MousePlayer) {
                                                        let (mouse_x, mouse_y) =
                                                            room.mouse_position;

                                                        let is_clickable = if mouse_x % 2 == 0 {
                                                            (x == mouse_x
                                                                && (y == mouse_y - 1
                                                                    || y == mouse_y + 1))
                                                                || (x == mouse_x - 1
                                                                    && y == mouse_y)
                                                                || (x == mouse_x + 1
                                                                    && y == mouse_y)
                                                                || (x == mouse_x - 1
                                                                    && y == mouse_y - 1)
                                                                || (x == mouse_x + 1
                                                                    && y == mouse_y - 1)
                                                        } else {
                                                            (x == mouse_x
                                                                && (y == mouse_y - 1
                                                                    || y == mouse_y + 1))
                                                                || (x == mouse_x - 1
                                                                    && y == mouse_y)
                                                                || (x == mouse_x + 1
                                                                    && y == mouse_y)
                                                                || (x == mouse_x - 1
                                                                    && y == mouse_y + 1)
                                                                || (x == mouse_x + 1
                                                                    && y == mouse_y + 1)
                                                        };

                                                        if is_clickable {
                                                            ui.painter().rect_filled(
                                                                button_response.rect,
                                                                egui::Rounding::same(5.0),
                                                                hover_color,
                                                            );
                                                        }
                                                    } else {
                                                        ui.painter().rect_filled(
                                                            button_response.rect,
                                                            egui::Rounding::same(5.0),
                                                            hover_color,
                                                        );
                                                    }
                                                }
                                            });
                                    }
                                });
                            }
                        });
                    });

                    ui.add_space(20.0);

                    if let Some(command) = command_to_send {
                        println!("Sending command: {}", command);
                        self.send_command(&command);
                    }

                    if ui.button("Back to Menu").clicked() {
                        if self.current_role == Some(Turn::MousePlayer) {
                            let command = format!("after_exit_room {} mouse ", room.room_id);
                            self.send_command(&command);
                        } else if self.current_role == Some(Turn::TrapperPlayer) {
                            let command = format!("after_exit_room {} trapper ", room.room_id);
                            self.send_command(&command);
                        }
                        let command = format!("game_over {} none ", room.room_id);
                        self.send_command(&command);
                        self.app_state = AppState::Menu;
                    }
                }
            }
        }
    }

    fn render_menu_single_player(&mut self, ui: &mut egui::Ui) {
        ui.add_space(80.0);
        ui.heading("Trap The Mouse!");
        ui.label(format!("Connected as: {} ", self.username));

        ui.add_space(30.0);

        ui.heading("Select Difficulty");
        ui.add_space(30.0);

        if ui.button("Easy").clicked() {
            let command = format!("set_difficulty easy !{} ", self.username);
            self.send_command(&command);

            self.app_state = AppState::InGameSinglePlayer;
        }
        ui.add_space(30.0);

        if ui.button("Normal").clicked() {
            let command = format!("set_difficulty medium !{} ", self.username);
            self.send_command(&command);

            self.app_state = AppState::InGameSinglePlayer;
        }
        ui.add_space(30.0);

        if ui.button("Back to Menu").clicked() {
            let command = format!("delete_room_by_name !{} ", self.username);
            self.send_command(&command);

            self.app_state = AppState::Menu;
        }
    }

    fn render_game_single_player(&mut self, ui: &mut egui::Ui) {
        let mut name: String = self.username.clone();
        name.insert(0, '!');
        if let Some(server_data) = &self.server_data.clone() {
            if let Some(room) = server_data.rooms.iter().find(|room| room.room_name == name) {
                self.current_room = Some(room.room_id);
                let current_role = self.current_role.clone();

                let (x, y) = room.mouse_position;
                let is_on_edge = x == 0 || x == 10 || y == 0 || y == 10;

                if is_on_edge {
                    let command = format!("game_over {} mouse ", room.room_id);
                    self.send_command(&command);
                    self.app_state = AppState::GameOver;
                } else if MyApp::is_surrounded(room.mouse_position, &room.walls) {
                    let command = format!("game_over {} trapper ", room.room_id);
                    self.app_state = AppState::GameOver;
                    self.send_command(&command);
                } else {
                    if room.game_state == GameState::GameOver {
                        self.app_state = AppState::GameOver;
                    }
                    let mut command_to_send: Option<String> = None;

                    ui.horizontal(|ui| {
                        ui.add_space(300.0);
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(">Mouse Player<");
                            ui.horizontal(|ui| {
                                ui.add_space(50.0);
                                ui.label("AI");
                            });
                        });
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(">Trapper Player<");
                            ui.horizontal(|ui| {
                                ui.add_space(50.0);
                                ui.label("You");
                            });
                        });
                    });

                    ui.add_space(10.0);
                    ui.label(">MOVING NOW<");

                    if current_role.as_ref().unwrap() == &room.turn {
                        ui.label("YOU");
                    } else {
                        ui.label("AI");
                    }

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.add_space(250.0);
                        ui.vertical(|ui| {
                            for x in 0..11 {
                                ui.horizontal(|ui| {
                                    if x % 2 == 1 {
                                        ui.add_space(15.0);
                                    }
                                    for y in 0..11 {
                                        let base_color = if (x, y) == room.mouse_position {
                                            Color32::from_gray(200)
                                        } else if room.walls.contains(&(x, y)) {
                                            Color32::from_rgb(255, 0, 0)
                                        } else {
                                            Color32::from_gray(100)
                                        };

                                        let hover_color = Color32::from_rgb(255, 165, 0);

                                        Frame::none()
                                            .fill(base_color)
                                            .rounding(egui::Rounding::same(5.0))
                                            .show(ui, |ui| {
                                                let button_response = ui.add(
                                                    Button::new("")
                                                        .frame(false)
                                                        .rounding(egui::Rounding::same(5.0))
                                                        .min_size(egui::vec2(20.0, 20.0)),
                                                );
                                                if button_response.clicked()
                                                    && (x, y) != room.mouse_position
                                                    && !room.walls.contains(&(x, y))
                                                {
                                                    if room.turn == Turn::TrapperPlayer {
                                                        command_to_send = Some(format!(
                                                            "place_trap {} {} {} ",
                                                            room.room_id, x, y
                                                        ));
                                                    }
                                                } else if room.turn == Turn::MousePlayer {
                                                    command_to_send =
                                                        Some(format!("AI_Move {} ", room.room_id));
                                                }

                                                if button_response.hovered()
                                                    && (x, y) != room.mouse_position
                                                    && !room.walls.contains(&(x, y))
                                                    && room.turn == Turn::TrapperPlayer
                                                {
                                                    ui.painter().rect_filled(
                                                        button_response.rect,
                                                        egui::Rounding::same(5.0),
                                                        hover_color,
                                                    );
                                                }
                                            });
                                    }
                                });
                            }
                        });
                    });

                    ui.add_space(20.0);

                    if let Some(command) = command_to_send {
                        println!("Sending command: {}", command);
                        self.send_command(&command);
                    }

                    // if ui.button("Game_Over").clicked() {
                    //     let command = format!("game_over {} none", room.room_id);
                    //     self.send_command(&command);
                    //     self.app_state = AppState::Menu;
                    // }
                }
            }
        }

        if ui.button("Back to Menu").clicked() {
            let command = format!("delete_room_by_name !{} ", self.username);
            self.send_command(&command);

            self.app_state = AppState::Menu;
        }
    }

    fn render_game_over(&mut self, ui: &mut egui::Ui) {
        if let Some(server_data) = &self.server_data.clone() {
            if let Some(room) = server_data
                .rooms
                .iter()
                .find(|room| Some(room.room_id) == self.current_room)
            {
                ui.add_space(250.0);

                if self.current_role == Some(Turn::MousePlayer) {
                    if room.winner == Some(Turn::MousePlayer) {
                        ui.heading("You Won!");
                    } else {
                        ui.heading("Try harder next time!");
                    }
                } else if room.winner == Some(Turn::TrapperPlayer) {
                        ui.heading("Congratulations You Won!");
                    } else if room.winner == Some(Turn::MousePlayer) {
                        ui.heading("Try harder next time!");
                    } else {
                        ui.heading("Your opponent got freaked out! You are really scary!");
                    }
                ui.add_space(100.0);
                if ui.button("Back to Menu").clicked() {
                    if room.room_type == RoomType::SinglePlayer {
                        let command = format!("delete_room {} ", room.room_id);
                        self.send_command(&command);
                        self.app_state = AppState::Menu;
                    }
                    if self.current_role == Some(Turn::MousePlayer) {
                        let command = format!("after_exit_room {} mouse ", room.room_id);
                        self.send_command(&command);
                        self.app_state = AppState::Menu;
                    } else if self.current_role == Some(Turn::TrapperPlayer) {
                        let command = format!("after_exit_room {} trapper ", room.room_id);
                        self.send_command(&command);
                        self.app_state = AppState::Menu;
                    }
                }
            }
        }
    }

    fn render_lobby(&mut self, ui: &mut egui::Ui) {
        ui.add_space(80.0);
        ui.heading("Trap The Mouse!");
        ui.label(format!("Connected as: {} ", self.username));

        ui.add_space(30.0);

        ui.heading("Lobby ");

        ui.add_space(30.0);

        if let Some(server_data) = &self.server_data {
            if let Some(room) = server_data
                .rooms
                .iter()
                .find(|room| Some(room.room_id) == self.current_room)
            {
                ui.heading(&room.room_name);
                ui.horizontal(|ui| {
                    ui.add_space(250.0);
                    ui.label("Mouse Player: ");
                    ui.label(
                        room.mouse_player
                            .as_ref()
                            .unwrap_or(&"Waiting for the other player".to_string()),
                    );
                });

                ui.horizontal(|ui| {
                    ui.add_space(250.0);
                    ui.label("Trapper Player:");
                    ui.label(
                        room.trapper_player
                            .as_ref()
                            .unwrap_or(&"Waiting for the other player".to_string()),
                    );
                });

                if room.trapper_player.is_some() && room.mouse_player.is_some() {
                    self.app_state = AppState::InGame;
                    // let command = format!("started_game {} ", room.room_id.clone());
                    // self.send_command(&command);
                }

                if ui.button("Back").clicked() {
                    ui.add_space(250.0);
                    let command = format!("exit_room {} {} ", room.room_id.clone(), self.username);
                    self.send_command(&command);
                    self.current_room = None;
                    self.app_state = AppState::Rooms;
                }
            }
        }
    }
}

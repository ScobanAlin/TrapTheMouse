use rand::prelude::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
#[derive(Serialize, Deserialize, PartialEq)]
enum GameState {
    Waiting,
    InGame,
    GameOver,
}

#[derive(Serialize, Deserialize, PartialEq)]
enum Turn {
    MousePlayer,
    TrapperPlayer,
}
#[derive(Serialize, Deserialize)]

enum RoomType {
    SinglePlayer,
    MultiPlayer,
}
#[derive(Serialize, Deserialize, PartialEq)]

enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Serialize, Deserialize)]
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

impl Room {
    pub fn new(room_id: u32, room_name: String) -> Self {
        let mut rng = rand::thread_rng();

        let walls = {
            let mut rand_walls = Vec::new();
            while rand_walls.len() < 6 {
                let x = rng.gen_range(0..11);
                let y = rng.gen_range(0..11);
                if (x, y) != (5, 5) && !rand_walls.contains(&(x, y)) {
                    rand_walls.push((x, y));
                }
            }
            rand_walls
        };

        Self {
            room_id,
            room_name,
            mouse_player: None,
            trapper_player: None,
            game_state: GameState::Waiting,
            mouse_position: (5, 5),
            walls,
            turn: Turn::TrapperPlayer,
            winner: None,
            room_type: RoomType::MultiPlayer,
            game_difficulty: None,
            mouse_player_exited: false,
            trapper_player_exited: false,
        }
    }

    pub fn ai_move(&mut self) {
        let (mouse_x, mouse_y) = self.mouse_position;
        let posib_moves = self.posib_moves(mouse_x, mouse_y);

        match self.game_difficulty.as_ref() {
            Some(Difficulty::Easy) => {
                if let Some(&(new_x, new_y)) = posib_moves.choose(&mut rand::thread_rng()) {
                    self.mouse_position = (new_x, new_y);
                    self.turn = Turn::TrapperPlayer;
                } else {
                    println!("No more moves!");
                    self.winner = Some(Turn::TrapperPlayer);
                }
            }
            Some(Difficulty::Medium) => {
                let medium_moves: Vec<(u32, u32)> = posib_moves
                    .iter()
                    .filter(|&&hex| !self.danger_hex(hex))
                    .cloned()
                    .collect();

                if !medium_moves.is_empty() {
                    if let Some(&(new_x, new_y)) = medium_moves.choose(&mut rand::thread_rng()) {
                        self.mouse_position = (new_x, new_y);
                        self.turn = Turn::TrapperPlayer;
                    } else {
                        println!("No more moves!");
                        self.winner = Some(Turn::TrapperPlayer);
                    }
                } else if let Some(&(new_x, new_y)) = posib_moves.choose(&mut rand::thread_rng()) {
                        self.mouse_position = (new_x, new_y);
                        self.turn = Turn::TrapperPlayer;
                    } else {
                        println!("No more moves!");
                        self.winner = Some(Turn::TrapperPlayer);
                    
                }
            }
            Some(Difficulty::Hard) => {}
            None => {
                println!("Vrajeala! nu are cum sa intre aici");
            }
        }
    }

    fn danger_hex(&self, hex: (u32, u32)) -> bool {
        let (x, y) = hex;
        if x > 0 && y > 0 {
            self.walls.iter().any(|&(wx, wy)| {
                (wx == x && (wy == y + 1 || wy == y - 1))
                    || (wy == y && (wx == x + 1 || wx == x - 1))
            })
        } else {
            false
        }
    }

    fn posib_moves(&self, mouse_x: u32, mouse_y: u32) -> Vec<(u32, u32)> {
        let direction_par: Vec<(i32, i32)> =
            vec![(0, 1), (0, -1), (1, -1), (1, 0), (-1, -1), (-1, 0)];
        let direction_impar: Vec<(i32, i32)> =
            vec![(0, 1), (0, -1), (1, 0), (1, 1), (-1, 0), (-1, 1)];
        let direction: &Vec<(i32, i32)> = if mouse_x % 2 == 0 {
            &direction_par
        } else {
            &direction_impar
        };

        let mut new_moves = Vec::new();

        for &(x, y) in direction {
            let new_x = mouse_x as i32 + x;
            let new_y = mouse_y as i32 + y;

            if new_x >= 0 && new_y >= 0 && new_x < 11 && new_y < 11 && !self.walls.contains(&(new_x as u32, new_y as u32)) {
                    new_moves.push((new_x as u32, new_y as u32));
                }
        }

        new_moves
    }
}

#[derive(Serialize, Deserialize)]
struct Server {
    rooms: Vec<Room>,
}

impl Server {
    pub fn new() -> Self {
        Self { rooms: Vec::new() }
    }

    pub fn create_room(&mut self, room_name: String) {
        let room_id = self.rooms.len() as u32 + 1;
        let new_room = Room::new(room_id, room_name);
        self.rooms.push(new_room);
    }
    pub fn create_single_room(&mut self, room_name: String) {
        let room_id = self.rooms.len() as u32 + 1;
        let mut new_room = Room::new(room_id, room_name);
        new_room.room_type = RoomType::SinglePlayer;
        self.rooms.push(new_room);
    }
}

fn handle_client(mut stream: TcpStream, server: Arc<Mutex<Server>>) {
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let message = String::from_utf8_lossy(&buffer[..n]);
                // println!("{}", message);
                if message.trim() != "get_update" {
                    println!("{}", message);
                } 

                if message.trim().contains("get_update") {
                    let server = server.lock().unwrap();
                    let serialized = serde_json::to_string(&*server).unwrap();
                    if stream.write_all(serialized.as_bytes()).is_err() {
                        break;
                    }
                } else if message.trim().starts_with("create_single_room") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_name = parts[1];
                    if let Ok(mut server) = server.lock() {
                        let mut x: String = room_name.to_string();
                        x.insert(0, '!');

                        server.create_single_room(x);
                    }
                } else if message.trim().starts_with("set_difficulty") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let difficulty = parts[1];
                    let room_name = parts[2];
                    if let Ok(mut server) = server.lock() {
                        if let Some(room) = server
                            .rooms
                            .iter_mut()
                            .find(|room| room.room_name == room_name)
                        {
                            room.game_difficulty = match difficulty {
                                "easy" => Some(Difficulty::Easy),
                                "medium" => Some(Difficulty::Medium),
                                "hard" => Some(Difficulty::Hard),
                                _ => None,
                            }
                        }
                    }
                } else if message.trim().starts_with("game_over") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_id: u32 = parts[1].parse().unwrap();
                    let winner = parts[2];
                    if let Ok(mut server) = server.lock() {
                        if let Some(room) =
                            server.rooms.iter_mut().find(|room| room.room_id == room_id)
                        {
                            room.game_state = GameState::GameOver;
                            room.winner = if winner == "trapper" {
                                Some(Turn::TrapperPlayer)
                            } else if winner == "none" {
                                None
                            } else {
                                Some(Turn::MousePlayer)
                            };
                        }
                    }
                } else if message.trim().starts_with("AI") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_id: u32 = parts[1].parse().unwrap();
                    if let Ok(mut server) = server.lock() {
                        if let Some(room) =
                            server.rooms.iter_mut().find(|room| room.room_id == room_id)
                        {
                            room.ai_move();
                        }
                    }
                } else if message.trim().starts_with("delete_room_by_name") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_name = parts[1];
                    if let Ok(mut server) = server.lock() {
                        server.rooms.retain(|room| room.room_name != room_name);
                    }
                } else if message.trim().starts_with("delete_room") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_id: u32 = parts[1].parse().unwrap();
                    if let Ok(mut server) = server.lock() {
                        server.rooms.retain(|room| room.room_id != room_id);
                    }
                } else if message.trim().starts_with("move_mouse") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_id: u32 = parts[1].parse().unwrap();
                    let x: u32 = parts[2].parse().unwrap();
                    let y: u32 = parts[3].parse().unwrap();
                    if let Ok(mut server) = server.lock() {
                        if let Some(room) =
                            server.rooms.iter_mut().find(|room| room.room_id == room_id)
                        {
                            room.mouse_position = (x, y);
                            room.turn = Turn::TrapperPlayer;
                        }
                    }
                } else if message.trim().starts_with("place_trap") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_id: u32 = parts[1].parse().unwrap();
                    let x: u32 = parts[2].parse().unwrap();
                    let y: u32 = parts[3].parse().unwrap();
                    if let Ok(mut server) = server.lock() {
                        if let Some(room) =
                            server.rooms.iter_mut().find(|room| room.room_id == room_id)
                        {
                            room.walls.push((x, y));
                            room.turn = Turn::MousePlayer;
                        }
                    }
                } else if message.trim().starts_with("create_room") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_name = parts[1];
                    if !room_name.is_empty() {
                        let mut server = server.lock().unwrap();
                        server.create_room(room_name.to_string().clone());
                    } 
                } else if message.trim().starts_with("join_room") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_id: u32 = parts[1].parse().unwrap();
                    let role = parts[2];
                    let username = parts[3];
                    if let Ok(mut server) = server.lock() {
                        if let Some(room) =
                            server.rooms.iter_mut().find(|room| room.room_id == room_id)
                        {
                            match role {
                                "mouse" => {
                                    room.mouse_player = Some(username.to_string());
                                }
                                "trapper" => {
                                    room.trapper_player = Some(username.to_string());
                                }
                                _ => {
                                    println!("ERR:role not correct");
                                }
                            }
                        }
                    }
                } else if message.trim().starts_with("after_exit_room") {
                    let parts: Vec<&str> = message.split_whitespace().collect();
                    let room_id: u32 = parts[1].parse().unwrap();
                    let username = parts[2];
                    if let Ok(mut server) = server.lock() {
                        if let Some(room) =
                            server.rooms.iter_mut().find(|room| room.room_id == room_id)
                        {


                            if username == "mouse" {
                                room.mouse_player_exited = true;
                            } else if username == "trapper" {
                                room.trapper_player_exited = true;
                            } 
                            room.game_state= GameState::GameOver;
                            if room.mouse_player_exited && room.trapper_player_exited
                            {
                                server.rooms.retain(|room| room.room_id != room_id);
                            }

                        }
                    }
                } else if message.trim().starts_with("exit_room") {
                    let parts: Vec<&str> = {
                        let this = &message;
                        this.trim_matches(|c: char| c.is_whitespace())
                    }.split_whitespace().collect();
                    let room_id: u32 = parts[1].parse().unwrap();
                    let username = parts[2];

                    if let Ok(mut server) = server.lock() {
                        if let Some(room) =
                            server.rooms.iter_mut().find(|room| room.room_id == room_id)
                        {
                            if room.mouse_player == Some(username.to_string()) {
                                room.mouse_player = None;
                            } else if room.trapper_player == Some(username.to_string()) {
                                room.trapper_player = None;
                            } else {
                                println!("ERR:Invalid username for exitting room.");
                            }
                        }
                    }
                } else if  stream.write_all(&buffer[..n]).is_err() {
                        break;
                    
                }
            }
            Err(_) => break,
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    let server = Arc::new(Mutex::new(Server::new()));
    listener.incoming().for_each(|stream| {
        if let Ok(stream) = stream {
            let server = Arc::clone(&server);
            thread::spawn(move || {
                handle_client(stream, server);
            });
        }
    });
    Ok(())
}

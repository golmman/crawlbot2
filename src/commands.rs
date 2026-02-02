use crate::logger::Logger;
use crate::map::MapState;
use crate::protocol::GameMessage;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum Routine {
    Idle,
    Hook1,
    Hook2,
    StartGame,
    StartSeededGame,
}

pub async fn execute_routine(
    routine: Routine,
    current: Option<&Value>,
    _next: Option<&Value>,
    map_state: &Arc<Mutex<MapState>>,
    logger: &Logger,
) -> (Routine, Vec<String>) {
    let msg = if let Some(current_val) = current {
        match serde_json::from_value::<GameMessage>(current_val.clone()) {
            Ok(m) => Some(m),
            Err(_) => return (routine, vec![]),
        }
    } else {
        None
    };

    let msg_type = msg.as_ref().map(|m| m.msg.as_str());
    let msg_title = msg.as_ref().and_then(|m| m.title.as_deref());

    logger
        .log(&format!(
            "[ROUTIN]: Executing routine with message '{:?}'\n",
            msg_type
        ))
        .await;

    if let Some(ref m) = msg {
        if m.msg == "map" {
            if let Some(cells) = &m.cells {
                let mut map = map_state.lock().await;
                map.update_map(cells, logger).await;
                let mut buf = Vec::new();
                if map.print_map(&mut buf).is_ok() {
                    if let Ok(s) = String::from_utf8(buf) {
                        logger.log(&s).await;
                    }
                }
            }
        }
    }

    logger
        .log(&format!(
            "[ROUTIN]: Executing {:?} routine logic\n",
            routine
        ))
        .await;

    match routine {
        Routine::Idle => (Routine::Idle, vec![]),
        Routine::Hook1 => (Routine::Idle, vec![]),
        Routine::Hook2 => (Routine::Idle, vec![]),
        Routine::StartGame => match msg_type {
            None => (Routine::StartGame, vec![command::register_random()]),
            Some("login_success") => (Routine::StartGame, vec![command::play()]),
            Some("ui-push") => match msg_title {
                Some(title) if title.contains("species") => {
                    (Routine::StartGame, vec![command::send_text("f")])
                }
                Some(title) if title.contains("background") => {
                    (Routine::StartGame, vec![command::send_text("f")])
                }
                Some(title) if title.contains("Welcome") => {
                    logger
                        .log("[ROUTIN]: StartSeededGame successfully finished\n")
                        .await;
                    (Routine::Idle, vec![command::send_text("f")])
                }
                _ => {
                    logger
                        .log("[ROUTIN]: StartSeededGame aborted, title not recognized\n")
                        .await;
                    (Routine::Idle, vec![])
                }
            },
            Some("html")
            | Some("set_game_links")
            | Some("game_client")
            | Some("game_started")
            | Some("chat")
            | Some("version")
            | Some("options")
            | Some("layout")
            | Some("ui-state-sync")
            | Some("ui-state")
            | Some("ui_state")
            | Some("ui-pop")
            | Some("player")
            | Some("update_spectators") => (Routine::StartGame, vec![]),
            _ => (Routine::Idle, vec![]),
        },
        Routine::StartSeededGame => match msg_type {
            None => (Routine::StartSeededGame, vec![command::register_random()]),
            Some("login_success") => (Routine::StartSeededGame, vec![command::play_seeded()]),
            Some("ui-push") => match msg_title {
                Some(title) if title.contains("Play a game with a custom seed") => (
                    Routine::StartSeededGame,
                    vec![
                        command::send_text("-"),
                        command::send_text("122333"),
                        command::send_keycode(13),
                    ],
                ),
                Some(title) if title.contains("Please select your species") => {
                    (Routine::StartSeededGame, vec![command::send_text("f")])
                }
                Some(title) if title.contains("Please select your background") => {
                    (Routine::StartSeededGame, vec![command::send_text("f")])
                }
                Some(title) if title.contains("Welcome") => {
                    logger
                        .log("[ROUTIN]: StartSeededGame successfully finished\n")
                        .await;
                    (Routine::Idle, vec![command::send_text("f")])
                }
                _ => {
                    logger
                        .log("[ROUTIN]: StartSeededGame aborted, title not recognized\n")
                        .await;
                    (Routine::Idle, vec![])
                }
            },
            Some("html")
            | Some("set_game_links")
            | Some("game_client")
            | Some("game_started")
            | Some("chat")
            | Some("version")
            | Some("options")
            | Some("layout")
            | Some("ui-state-sync")
            | Some("ui-state")
            | Some("ui_state")
            | Some("ui-pop")
            | Some("player")
            | Some("text_cursor")
            | Some("update_spectators") => (Routine::StartSeededGame, vec![]),
            _ => (Routine::Idle, vec![]),
        },
    }
}

pub async fn handle_repl_command(command: &str, logger: &Logger) -> (Routine, Vec<String>) {
    logger
        .log(&format!("[REPL  ]: handling repl command '{}'\n", command))
        .await;

    match command {
        "/hook1" => (Routine::Hook1, vec![]),
        "/hook2" => (Routine::Hook2, vec![]),
        "/start" => (Routine::StartGame, vec![]),
        "/seeded" => (Routine::StartSeededGame, vec![]),
        _ => {
            logger
                .log(&format!("unknown repl command: {}\n", command))
                .await;
            (Routine::Idle, vec![command.to_string()])
        }
    }
}

mod command {
    use chrono::Local;
    use serde_json::json;

    pub fn register_random() -> String {
        let now = Local::now();
        let username = format!("dirkle{}", now.format("%Y%m%d%H%M%S"));
        json!({
            "msg": "register",
            "username": username,
            "password": "aaa",
            "email": ""
        })
        .to_string()
    }

    pub fn play() -> String {
        json!({"msg":"play","game_id":"dcss-web-trunk"}).to_string()
    }

    pub fn play_seeded() -> String {
        json!({"msg":"play","game_id":"seeded-web-trunk"}).to_string()
    }

    pub fn send_text(text: &str) -> String {
        json!({"msg": "input", "text": text}).to_string()
    }

    pub fn send_keycode(keycode: i32) -> String {
        json!({"msg":"key","keycode": keycode}).to_string()
    }

    #[allow(dead_code)]
    pub fn register() -> String {
        json!({
            "msg": "register",
            "username": "dirkle",
            "password": "aaa",
            "email": ""
        })
        .to_string()
    }

    #[allow(dead_code)]
    pub fn login() -> String {
        json!({"msg":"login","username":"dirkle","password":"aaa"}).to_string()
    }
}

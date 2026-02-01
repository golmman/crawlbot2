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
    StartSeededGame,
}

pub async fn execute_routine(
    routine: &mut Routine,
    current: Option<&Value>,
    _next: Option<&Value>,
    map_state: &Arc<Mutex<MapState>>,
    logger: &Logger,
) -> Option<String> {
    let msg = if let Some(current_val) = current {
        match serde_json::from_value::<GameMessage>(current_val.clone()) {
            Ok(m) => Some(m),
            Err(_) => return None,
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

    match routine {
        Routine::Idle => {
            logger.log("[ROUTIN]: Executing Idle routine logic\n").await;
            *routine = Routine::Idle;
            None
        }
        Routine::Hook1 => {
            logger
                .log("[ROUTIN]: Executing Hook1 routine logic\n")
                .await;
            *routine = Routine::Idle;
            None
        }
        Routine::Hook2 => {
            logger
                .log("[ROUTIN]: Executing Hook2 routine logic\n")
                .await;
            *routine = Routine::Idle;
            None
        }
        Routine::StartSeededGame => {
            logger
                .log("[ROUTIN]: Executing StartSeededGame routine logic\n")
                .await;

            let result = match msg_type {
                None => {
                    *routine = Routine::StartSeededGame;
                    command::register_random()
                }
                Some("login_success") => {
                    *routine = Routine::StartSeededGame;
                    command::play()
                }
                Some("ui-push") => {
                    if let Some(msg_title) = msg_title {
                        if msg_title.contains("species") {
                            command::press("f")
                        } else if msg_title.contains("background") {
                            command::press("f")
                        } else {
                            logger
                                .log("[ROUTIN]: StartSeededGame aborted, title not recognized\n")
                                .await;
                            *routine = Routine::Idle;
                            None
                        }
                    } else {
                        logger
                            .log("[ROUTIN]: StartSeededGame aborted, title not recognized\n")
                            .await;
                        *routine = Routine::Idle;
                        None
                    }
                }
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
                | Some("player")
                | Some("update_spectators") => {
                    *routine = Routine::StartSeededGame;
                    None
                }
                _ => {
                    *routine = Routine::Idle;
                    None
                }
            };

            result
        }
    }
}

pub async fn handle_repl_command(command: &str, logger: &Logger) -> (Routine, Option<String>) {
    logger
        .log(&format!("[REPL  ]: handling repl command '{}'\n", command))
        .await;

    match command {
        "/hook1" => (Routine::Hook1, None),
        "/hook2" => (Routine::Hook2, None),
        "/start" => (Routine::StartSeededGame, None),
        _ => {
            logger
                .log(&format!("unknown repl command: {}\n", command))
                .await;
            (Routine::Idle, Some(command.to_string()))
        }
    }
}

mod command {
    use chrono::Local;
    use serde_json::json;

    pub fn register_random() -> Option<String> {
        let now = Local::now();
        let username = format!("dirkle{}", now.format("%Y%m%d%H%M%S"));
        Some(
            json!({
                "msg": "register",
                "username": username,
                "password": "aaa",
                "email": ""
            })
            .to_string(),
        )
    }

    pub fn play() -> Option<String> {
        Some(json!({"msg":"play","game_id":"dcss-web-trunk"}).to_string())
    }

    pub fn press(key: &str) -> Option<String> {
        Some(json!({"msg": "input","text": key}).to_string())
    }

    pub fn register() -> Option<String> {
        Some(
            json!({
                "msg": "register",
                "username": "dirkle",
                "password": "aaa",
                "email": ""
            })
            .to_string(),
        )
    }

    pub fn login() -> Option<String> {
        Some(json!({"msg":"login","username":"dirkle","password":"aaa"}).to_string())
    }
}

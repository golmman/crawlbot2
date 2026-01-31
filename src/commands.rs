use crate::logger::Logger;
use crate::map::MapState;
use crate::protocol::GameMessage;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum Routine {
    Idle,
    Hook1,
    Hook2,
}

pub struct MessageHook {
    pub current_routine: Routine,
}

impl MessageHook {
    pub fn new() -> Self {
        Self {
            current_routine: Routine::Idle,
        }
    }
}

pub async fn execute_routine(
    routine: &mut Routine,
    current: &Value,
    _next: Option<&Value>,
    map_state: &Arc<Mutex<MapState>>,
    logger: &Logger,
) -> Option<String> {
    // Basic game message processing (like map updates) should happen here too
    if let Ok(msg) = serde_json::from_value::<GameMessage>(current.clone()) {
        if msg.msg == "map" {
            if let Some(cells) = &msg.cells {
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
        Routine::Idle => None,
        Routine::Hook1 => {
            logger.log("Executing Hook1 routine logic\n").await;
            // Example: after one message, go back to idle
            *routine = Routine::Idle;
            None
        }
        Routine::Hook2 => {
            logger.log("Executing Hook2 routine logic\n").await;
            *routine = Routine::Idle;
            None
        }
    }
}

pub fn handle_repl_command(command: &str, _hook: &mut MessageHook) -> (Routine, Option<String>) {
    match command {
        "/hook1" => (Routine::Hook1, None),
        "/hook2" => (Routine::Hook2, None),
        "/start" => (
            Routine::Idle,
            Some(
                json!({
                    "msg": "register",
                    "username": "dirkle",
                    "password": "aaa",
                    "email": ""
                })
                .to_string(),
            ),
        ),
        _ => (Routine::Idle, Some(command.to_string())),
    }
}

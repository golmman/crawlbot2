use crate::logger::Logger;
use crate::map::MapState;
use crate::protocol::GameMessage;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

const REGISTER: Option<&str> =
    Some(r#"{"msg":"register","username":"dirkle","password":"aaa","email":""}"#);

#[derive(Debug, Clone, PartialEq)]
pub enum Routine {
    Idle,
    Hook1,
    Hook2,
    StartSeededGame,
}

pub async fn execute_routine(
    routine: &mut Routine,
    current: &Value,
    _next: Option<&Value>,
    map_state: &Arc<Mutex<MapState>>,
    logger: &Logger,
) -> Option<String> {
    let msg = match serde_json::from_value::<GameMessage>(current.clone()) {
        Ok(msg) => msg,
        Err(_) => return None,
    };

    logger
        .log(&format!(
            "[ROUTIN]: Executing routine with message '{}'\n",
            msg.msg
        ))
        .await;

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

            if 1 == 1 {
                return REGISTER.map(String::from);
            }

            *routine = Routine::Idle;
            None
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
        "/start" => (Routine::StartSeededGame, REGISTER.map(String::from)),
        _ => {
            logger
                .log(&format!("unknown repl command: {}\n", command))
                .await;
            (Routine::Idle, Some(command.to_string()))
        }
    }
}

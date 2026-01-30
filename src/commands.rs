use crate::logger::Logger;
use serde_json::json;

pub struct MessageHook {
    #[allow(dead_code)]
    pub callback: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
}

impl MessageHook {
    pub fn new() -> Self {
        Self { callback: None }
    }
}

pub async fn create_message(command: &str, hook: &mut MessageHook, logger: &Logger) -> String {
    match command {
        "/hook1" => hook1(hook, logger).await,
        "/hook2" => hook2(hook, logger).await,
        "/start" => start(hook),
        cmd => {
            logger.log(&format!("unknown command: {}\n", cmd)).await;
            String::new()
        }
    }
}

async fn hook1(_hook: &mut MessageHook, logger: &Logger) -> String {
    logger.log("hook1\n").await;
    // In a real implementation with callbacks, we'd set _hook.callback here.
    // For this port, we'll just mimic the logic.
    String::new()
}

async fn hook2(_hook: &mut MessageHook, logger: &Logger) -> String {
    logger.log("hook2\n").await;
    String::new()
}

fn start(_hook: &mut MessageHook) -> String {
    json!({
        "msg": "register",
        "username": "dirkle",
        "password": "aaa",
        "email": ""
    })
    .to_string()
}

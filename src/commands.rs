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

pub fn create_message(command: &str, hook: &mut MessageHook) -> String {
    match command {
        "/hook1" => hook1(hook),
        "/hook2" => hook2(hook),
        "/start" => start(hook),
        cmd => {
            println!("unknown command: {}", cmd);
            String::new()
        }
    }
}

fn hook1(_hook: &mut MessageHook) -> String {
    println!("hook1");
    // In a real implementation with callbacks, we'd set _hook.callback here.
    // For this port, we'll just mimic the logic.
    String::new()
}

fn hook2(_hook: &mut MessageHook) -> String {
    println!("hook2");
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

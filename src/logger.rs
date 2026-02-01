use chrono::Local;
use rustyline_async::SharedWriter;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

#[derive(Clone)]
pub struct Logger {
    stdout: SharedWriter,
    file: std::sync::Arc<tokio::sync::Mutex<File>>,
}

impl Logger {
    pub async fn new(stdout: SharedWriter) -> Result<Self, Box<dyn std::error::Error>> {
        let logs_dir = Path::new("./logs");
        if !logs_dir.exists() {
            fs::create_dir_all(logs_dir)?;
        }

        let now = Local::now();
        let filename = format!("log-{}.txt", now.format("%Y%m%dT%H%M%S"));
        let file_path = logs_dir.join(filename);
        let file = File::create(file_path)?;

        Ok(Self {
            stdout,
            file: std::sync::Arc::new(tokio::sync::Mutex::new(file)),
        })
    }

    pub async fn log(&self, message: &str) {
        let now = Local::now();
        let timestamped_message = format!("{} {}", now.format("%Y-%m-%dT%H:%M:%S"), message);

        // Write to stdout (SharedWriter)
        let mut stdout = self.stdout.clone();
        let _ = stdout.write_all(timestamped_message.as_bytes());
        let _ = stdout.flush();

        // Write to file
        let mut file = self.file.lock().await;
        let _ = file.write_all(timestamped_message.as_bytes());
        let _ = file.flush();
    }
}

mod commands;
mod map;

use commands::{create_message, MessageHook};
use map::{Cell, MapState};

use flate2::{Decompress, FlushDecompress};
use futures_util::{SinkExt, StreamExt};
use rustyline_async::{Readline, ReadlineEvent};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use futures_util::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Debug, Deserialize, Serialize)]
struct GameMessage {
    msg: String,
    cells: Option<Vec<Cell>>,
    #[serde(flatten)]
    other: serde_json::Map<String, Value>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url_str = "ws://127.0.0.1:8080/socket";
    let (ws_stream, _) = connect_async(url_str).await?;
    println!("Connected. Forcing Manual Decompression...");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let map_state = Arc::new(Mutex::new(MapState::new()));
    let message_hook = Arc::new(Mutex::new(MessageHook::new()));

    let (mut rl, mut stdout) = Readline::new(format!(
        "{} DCSS    > ",
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S")
    ))?;

    let map_state_clone = Arc::clone(&map_state);
    let mut decompressor = Decompress::new(false); // raw deflate

    // Channel for sending messages to the WebSocket
    let (tx, mut rx) = mpsc::channel::<Message>(32);

    // WebSocket sender task
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                eprintln!("WebSocket send error: {:?}", e);
                break;
            }
        }
    });

    let tx_clone = tx.clone();
    let mut stdout_clone = stdout.clone();
    // WebSocket receiver task
    tokio::spawn(async move {
        let mut buffer = Vec::new();
        let sync_buffer = [0x00, 0x00, 0xff, 0xff];

        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    let mut input = data.to_vec();
                    input.extend_from_slice(&sync_buffer);
                    let mut offset = 0;

                    loop {
                        let prev_in = decompressor.total_in();
                        let prev_out = decompressor.total_out();

                        let mut temp_buffer = vec![0u8; 32768];
                        let res = decompressor.decompress(
                            &input[offset..],
                            &mut temp_buffer,
                            FlushDecompress::Sync,
                        );

                        let consumed = (decompressor.total_in() - prev_in) as usize;
                        let produced = (decompressor.total_out() - prev_out) as usize;

                        offset += consumed;
                        buffer.extend_from_slice(&temp_buffer[..produced]);

                        match res {
                            Ok(flate2::Status::Ok) | Ok(flate2::Status::BufError) => {
                                if consumed == 0 && produced == 0 {
                                    break;
                                }
                            }
                            Ok(flate2::Status::StreamEnd) => break,
                            Err(e) => {
                                let _ = stdout_clone.write_all(format!("Decompression error: {:?}\n", e).as_bytes()).await;
                                break;
                            }
                        }

                        if offset >= input.len() {
                            break;
                        }
                    }

                    if let Ok(json_data) = String::from_utf8(buffer.clone()) {
                        // Use StreamDeserializer to handle one or more JSON values in the buffer
                        let mut stream = serde_json::Deserializer::from_str(&json_data).into_iter::<Value>();
                        let mut last_offset = 0;

                        while let Some(Ok(value)) = stream.next() {
                            last_offset = stream.byte_offset();
                            let _ = stdout_clone.write_all(format!("\n{} [Server Raw]: {}\n", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"), value).as_bytes()).await;

                            // The TS client expects an array of messages
                            if let Some(messages) = value.as_array() {
                                for msg_val in messages {
                                    if let Ok(msg) = serde_json::from_value::<GameMessage>(msg_val.clone()) {
                                        if msg.msg == "map" {
                                            if let Some(cells) = &msg.cells {
                                                let map_buffer = {
                                                    let mut map = map_state_clone.lock().unwrap();
                                                    map.update_map(cells);
                                                    let mut buf = Vec::new();
                                                    if map.print_map(&mut buf).is_ok() {
                                                        Some(buf)
                                                    } else {
                                                        None
                                                    }
                                                };
                                                if let Some(buf) = map_buffer {
                                                    let _ = stdout_clone.write_all(&buf).await;
                                                }
                                            }
                                        }

                                        if msg.msg == "ping" {
                                            let tx_inner = tx_clone.clone();
                                            let mut stdout_inner = stdout_clone.clone();
                                            tokio::spawn(async move {
                                                sleep(Duration::from_secs(5)).await;
                                                let _ = tx_inner.send(Message::Text(r#"{"msg":"pong"}"#.into())).await;
                                                let _ = stdout_inner.write_all(format!("\n{} [Client]: pong message sent\n", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S")).as_bytes()).await;
                                            });
                                        }
                                    }
                                }
                            } else {
                                // Maybe it's a single message?
                                if let Ok(msg) = serde_json::from_value::<GameMessage>(value.clone()) {
                                    if msg.msg == "map" {
                                        if let Some(cells) = &msg.cells {
                                            let map_buffer = {
                                                let mut map = map_state_clone.lock().unwrap();
                                                map.update_map(cells);
                                                let mut buf = Vec::new();
                                                if map.print_map(&mut buf).is_ok() {
                                                    Some(buf)
                                                } else {
                                                    None
                                                }
                                            };
                                            if let Some(buf) = map_buffer {
                                                let _ = stdout_clone.write_all(&buf).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Remove only the part of the buffer that was successfully parsed
                        if last_offset > 0 {
                            buffer.drain(..last_offset);
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    let _ = stdout_clone.write_all(format!("WebSocket error: {:?}\n", e).as_bytes()).await;
                    break;
                }
                _ => {}
            }
        }
    });

    // REPL loop
    loop {
        match rl.readline().await {
            Ok(ReadlineEvent::Line(line)) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if line.starts_with('/') {
                    let mut hook = message_hook.lock().unwrap();
                    let msg = create_message(line, &mut hook);
                    if !msg.is_empty() {
                        tx.send(Message::Text(msg.into())).await?;
                    }
                } else {
                    tx.send(Message::Text(line.into())).await?;
                }
                rl.add_history_entry(line.to_string());
            }
            Ok(ReadlineEvent::Eof) | Ok(ReadlineEvent::Interrupted) => break,
            Err(e) => {
                let _ = stdout.write_all(format!("Readline error: {:?}\n", e).as_bytes()).await;
                break;
            }
        }
    }

    Ok(())
}

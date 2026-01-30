mod commands;
mod map;
mod protocol;

use commands::{MessageHook, create_message};
use map::MapState;
use protocol::{GameMessage, normalize_messages};

use flate2::{Decompress, FlushDecompress};
use futures_util::AsyncWriteExt;
use futures_util::{SinkExt, StreamExt};
use rustyline_async::{Readline, ReadlineEvent, SharedWriter};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

type WsSender = futures_util::stream::SplitSink<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;
type WsReceiver = futures_util::stream::SplitStream<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url_str = "ws://127.0.0.1:8080/socket";
    let (ws_stream, _) = connect_async(url_str).await?;
    println!("Connected. Forcing Manual Decompression...");

    let (ws_sender, ws_receiver) = ws_stream.split();
    let map_state = Arc::new(Mutex::new(MapState::new()));
    let message_hook = Arc::new(Mutex::new(MessageHook::new()));

    let (rl, stdout) = Readline::new(format!(
        "{} DCSS    > ",
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S")
    ))?;

    // Channel for sending messages to the WebSocket
    let (tx, rx) = mpsc::channel::<Message>(32);

    spawn_sender(ws_sender, rx);
    spawn_receiver(
        ws_receiver,
        Arc::clone(&map_state),
        tx.clone(),
        stdout.clone(),
    );

    run_repl(rl, stdout, tx, message_hook).await?;

    Ok(())
}

fn spawn_sender(mut ws_sender: WsSender, mut rx: mpsc::Receiver<Message>) {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                eprintln!("WebSocket send error: {:?}", e);
                break;
            }
        }
    });
}

fn spawn_receiver(
    mut ws_receiver: WsReceiver,
    map_state: Arc<Mutex<MapState>>,
    tx: mpsc::Sender<Message>,
    mut stdout: SharedWriter,
) {
    tokio::spawn(async move {
        let mut buffer = Vec::new();
        let sync_buffer = [0x00, 0x00, 0xff, 0xff];
        let mut decompressor = Decompress::new(false); // raw deflate

        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    let res = handle_binary_message(
                        data.to_vec(),
                        &sync_buffer,
                        &mut decompressor,
                        &mut buffer,
                        &map_state,
                        &tx,
                        &mut stdout,
                    )
                    .await;

                    if let Err(e) = res {
                        let err_msg = format!("Error handling message: {:?}\n", e);
                        let _ = stdout.write_all(err_msg.as_bytes()).await;
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    let _ = stdout
                        .write_all(format!("WebSocket error: {:?}\n", e).as_bytes())
                        .await;
                    break;
                }
                _ => {}
            }
        }
    });
}

async fn handle_binary_message(
    data: Vec<u8>,
    sync_buffer: &[u8],
    decompressor: &mut Decompress,
    buffer: &mut Vec<u8>,
    map_state: &Arc<Mutex<MapState>>,
    tx: &mpsc::Sender<Message>,
    stdout: &mut SharedWriter,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut input = data;
    input.extend_from_slice(sync_buffer);
    let mut offset = 0;

    loop {
        let prev_in = decompressor.total_in();
        let prev_out = decompressor.total_out();

        let mut temp_buffer = vec![0u8; 32768];
        let res =
            decompressor.decompress(&input[offset..], &mut temp_buffer, FlushDecompress::Sync);

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
            Err(e) => return Err(e.into()),
        }

        if offset >= input.len() {
            break;
        }
    }

    if let Ok(json_data) = String::from_utf8(buffer.clone()) {
        let mut stream = serde_json::Deserializer::from_str(&json_data).into_iter::<Value>();
        let mut last_offset = 0;

        while let Some(Ok(value)) = stream.next() {
            last_offset = stream.byte_offset();
            let _ = stdout
                .write_all(
                    format!(
                        "\n{} [Server Raw]: {}\n",
                        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                        value
                    )
                    .as_bytes(),
                )
                .await;

            for msg_val in normalize_messages(value) {
                process_game_message(msg_val, map_state, tx, stdout).await?;
            }
        }

        if last_offset > 0 {
            buffer.drain(..last_offset);
        }
    }

    Ok(())
}

async fn process_game_message(
    msg_val: Value,
    map_state: &Arc<Mutex<MapState>>,
    tx: &mpsc::Sender<Message>,
    stdout: &mut SharedWriter,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match serde_json::from_value::<GameMessage>(msg_val.clone()) {
        Ok(msg) => {
            if msg.msg == "map" {
                if let Some(cells) = &msg.cells {
                    let map_buffer = {
                        let mut map = map_state.lock().unwrap();
                        map.update_map(cells);
                        let mut buf = Vec::new();
                        if map.print_map(&mut buf).is_ok() {
                            Some(buf)
                        } else {
                            None
                        }
                    };
                    if let Some(buf) = map_buffer {
                        let _ = stdout.write_all(&buf).await;
                    }
                }
            }

            if msg.msg == "ping" {
                let tx_inner = tx.clone();
                let mut stdout_inner = stdout.clone();
                tokio::spawn(async move {
                    sleep(Duration::from_secs(5)).await;
                    let _ = tx_inner
                        .send(Message::Text(r#"{"msg":"pong"}"#.into()))
                        .await;
                    let _ = stdout_inner
                        .write_all(
                            format!(
                                "\n{} [Client]: pong message sent\n",
                                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S")
                            )
                            .as_bytes(),
                        )
                        .await;
                });
            }
        }
        Err(e) => {
            if let Some(m) = msg_val.get("msg").and_then(|m| m.as_str()) {
                if m != "map" && m != "ping" {
                    // ignore
                }
            } else {
                let _ = stdout
                    .write_all(
                        format!("Failed to parse GameMessage from {:?}: {:?}\n", msg_val, e)
                            .as_bytes(),
                    )
                    .await;
            }
        }
    }
    Ok(())
}

async fn run_repl(
    mut rl: Readline,
    mut stdout: SharedWriter,
    tx: mpsc::Sender<Message>,
    message_hook: Arc<Mutex<MessageHook>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
                let _ = stdout
                    .write_all(format!("Readline error: {:?}\n", e).as_bytes())
                    .await;
                break;
            }
        }
    }
    Ok(())
}

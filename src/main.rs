mod commands;
mod logger;
mod map;
mod protocol;

use crate::protocol::normalize_messages;
use commands::Routine;
use flate2::{Decompress, FlushDecompress};
use futures_util::SinkExt;
use futures_util::StreamExt;
use logger::Logger;
use map::MapState;
use rustyline_async::{Readline, ReadlineEvent};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
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

    let (ws_sender, ws_receiver) = ws_stream.split();
    let map_state = Arc::new(Mutex::new(MapState::new()));
    let current_routine = Arc::new(Mutex::new(Routine::Idle));

    let (rl, stdout) = Readline::new("DCSS    > ".to_string())?;

    let logger = Logger::new(stdout).await?;
    logger
        .log("Connected. Forcing Manual Decompression...\n")
        .await;

    // Channel for sending messages to the WebSocket
    let (tx_sender, rx_sender) = mpsc::channel::<Message>(32);
    // Channel for incoming messages (Server + Repl)
    let (tx_receiver, rx_receiver) = mpsc::channel::<protocol::ProcessMessage>(32);

    spawn_sender(ws_sender, rx_sender, logger.clone());
    spawn_receiver(ws_receiver, tx_receiver.clone(), logger.clone());

    spawn_processor(
        rx_receiver,
        tx_sender,
        Arc::clone(&map_state),
        Arc::clone(&current_routine),
        logger.clone(),
    );

    run_repl(rl, logger, tx_receiver, current_routine).await?;

    Ok(())
}

fn spawn_sender(mut ws_sender: WsSender, mut rx: mpsc::Receiver<Message>, logger: Logger) {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            sleep(Duration::from_millis(2500)).await;
            logger.log(&format!("[CLIENT]: {}\n", msg)).await;
            if let Err(e) = ws_sender.send(msg).await {
                eprintln!("WebSocket send error: {:?}", e);
                break;
            }
        }
    });
}

fn spawn_receiver(
    mut ws_receiver: WsReceiver,
    tx_receiver: mpsc::Sender<protocol::ProcessMessage>,
    logger: Logger,
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
                        &tx_receiver,
                        &logger,
                    )
                    .await;

                    if let Err(e) = res {
                        let err_msg = format!("Error handling message: {:?}\n", e);
                        logger.log(&err_msg).await;
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    logger.log(&format!("WebSocket error: {:?}\n", e)).await;
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
    tx_receiver: &mpsc::Sender<protocol::ProcessMessage>,
    logger: &Logger,
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
            logger.log(&format!("[SERVER]: {}\n", value)).await;

            for msg_val in normalize_messages(value) {
                tx_receiver
                    .send(protocol::ProcessMessage::Server(msg_val))
                    .await?;
            }
        }

        if last_offset > 0 {
            buffer.drain(..last_offset);
        }
    }

    Ok(())
}

fn spawn_processor(
    mut rx_receiver: mpsc::Receiver<protocol::ProcessMessage>,
    tx_sender: mpsc::Sender<Message>,
    map_state: Arc<Mutex<MapState>>,
    current_routine: Arc<Mutex<Routine>>,
    logger: Logger,
) {
    tokio::spawn(async move {
        let mut peeked: Option<protocol::ProcessMessage> = None;

        loop {
            let msg = if let Some(m) = peeked.take() {
                Some(m)
            } else {
                rx_receiver.recv().await
            };

            let Some(msg) = msg else { break };

            match msg {
                protocol::ProcessMessage::Repl(line) => {
                    let mut routine = current_routine.lock().await;
                    let (new_routine, outgoing) =
                        commands::handle_repl_command(&line, &logger).await;
                    *routine = new_routine;

                    let mut messages = outgoing;
                    if messages.is_empty() {
                        messages = commands::execute_routine(
                            &mut *routine,
                            None,
                            None,
                            &map_state,
                            &logger,
                        )
                        .await;
                    }

                    for msg_str in messages {
                        let _ = tx_sender.send(Message::Text(msg_str.into())).await;
                    }
                }
                protocol::ProcessMessage::Server(val) => {
                    // Check for ping
                    if val.get("msg").and_then(|m| m.as_str()) == Some("ping") {
                        let tx_inner = tx_sender.clone();
                        tokio::spawn(async move {
                            let _ = tx_inner
                                .send(Message::Text(r#"{"msg":"pong"}"#.into()))
                                .await;
                        });
                        continue;
                    }

                    // Process with current routine
                    // Manual peek:
                    peeked = match rx_receiver.try_recv() {
                        Ok(p) => Some(p),
                        Err(_) => None,
                    };
                    let next_val = match &peeked {
                        Some(protocol::ProcessMessage::Server(v)) => Some(v),
                        _ => None,
                    };
                    let mut routine = current_routine.lock().await;

                    let outgoing = commands::execute_routine(
                        &mut *routine,
                        Some(&val),
                        next_val,
                        &map_state,
                        &logger,
                    )
                    .await;

                    for msg_str in outgoing {
                        let _ = tx_sender.send(Message::Text(msg_str.into())).await;
                    }
                }
            }
        }
    });
}

async fn run_repl(
    mut rl: Readline,
    logger: Logger,
    tx_receiver: mpsc::Sender<protocol::ProcessMessage>,
    _current_routine: Arc<Mutex<Routine>>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match rl.readline().await {
            Ok(ReadlineEvent::Line(line)) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if line.starts_with('/') {
                    tx_receiver
                        .send(protocol::ProcessMessage::Repl(line.to_string()))
                        .await?;
                } else {
                    tx_receiver
                        .send(protocol::ProcessMessage::Repl(line.to_string()))
                        .await?;
                }
                rl.add_history_entry(line.to_string());
            }
            Ok(ReadlineEvent::Eof) | Ok(ReadlineEvent::Interrupted) => break,
            Err(e) => {
                logger.log(&format!("Readline error: {:?}\n", e)).await;
                break;
            }
        }
    }
    Ok(())
}

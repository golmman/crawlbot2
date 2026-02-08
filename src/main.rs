mod commands;
mod logger;
mod map;
mod protocol;

use crate::protocol::{GameMessage, Routine, RoutineMessage, normalize_messages};
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

const SENDER_DELAY_MS: u64 = 100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url_str = "ws://127.0.0.1:8080/socket";
    let (ws_stream, _) = connect_async(url_str).await?;

    let (ws_sender, ws_receiver) = ws_stream.split();
    let map_state = Arc::new(Mutex::new(MapState::new()));
    let current_routine = Arc::new(Mutex::new(Routine::Init));

    let (rl, stdout) = Readline::new("DCSS    > ".to_string())?;

    let logger = Logger::new(stdout).await?;
    logger
        .log("Connected. Forcing Manual Decompression...\n")
        .await;

    // 1. Game Server Output Channel
    let (output_tx, output_rx) = mpsc::channel::<Message>(32);
    
    // 2. Routine Channel
    let (routine_tx, routine_rx) = mpsc::channel::<RoutineMessage>(32);

    // Spawn Game Server Output Handler
    spawn_output_handler(ws_sender, output_rx, logger.clone());

    // Spawn Game Server Input Handler
    spawn_input_handler(
        ws_receiver,
        output_tx.clone(),
        routine_tx.clone(),
        logger.clone(),
    );

    // Spawn Routine Handler
    spawn_routine_handler(
        routine_rx,
        routine_tx.clone(),
        output_tx.clone(),
        Arc::clone(&map_state),
        Arc::clone(&current_routine),
        logger.clone(),
    );

    // Run Repl Handler
    run_repl(rl, logger, output_tx, routine_tx).await?;

    Ok(())
}

fn spawn_output_handler(mut ws_sender: WsSender, mut rx: mpsc::Receiver<Message>, logger: Logger) {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            sleep(Duration::from_millis(SENDER_DELAY_MS)).await;
            logger.log(&format!("[CLIENT]: {}\n", msg)).await;
            if let Err(e) = ws_sender.send(msg).await {
                eprintln!("WebSocket send error: {:?}", e);
                break;
            }
        }
    });
}

fn spawn_input_handler(
    mut ws_receiver: WsReceiver,
    output_tx: mpsc::Sender<Message>,
    routine_tx: mpsc::Sender<RoutineMessage>,
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
                        &output_tx,
                        &routine_tx,
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
    output_tx: &mpsc::Sender<Message>,
    routine_tx: &mpsc::Sender<RoutineMessage>,
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
                // Check if it is a ping message
                if msg_val.get("msg").and_then(|m| m.as_str()) == Some("ping") {
                    let _ = output_tx.send(Message::Text(r#"{"msg":"pong"}"#.into())).await;
                } else {
                    // Try to deserialize to GameMessage
                    match serde_json::from_value::<GameMessage>(msg_val) {
                        Ok(game_msg) => {
                            let _ = routine_tx.send(RoutineMessage::GameMessage(game_msg)).await;
                        }
                        Err(e) => {
                            logger.log(&format!("Failed to parse game message: {:?}\n", e)).await;
                        }
                    }
                }
            }
        }

        if last_offset > 0 {
            buffer.drain(..last_offset);
        }
    }

    Ok(())
}

fn spawn_routine_handler(
    mut routine_rx: mpsc::Receiver<RoutineMessage>,
    routine_tx: mpsc::Sender<RoutineMessage>,
    output_tx: mpsc::Sender<Message>,
    map_state: Arc<Mutex<MapState>>,
    current_routine: Arc<Mutex<Routine>>,
    logger: Logger,
) {
    tokio::spawn(async move {
        while let Some(msg) = routine_rx.recv().await {
            match msg {
                RoutineMessage::Override(new_routine) => {
                    let mut routine_store = current_routine.lock().await;
                    *routine_store = new_routine;
                    
                    // Execute immediately after override
                    let (next_routine_opt, outgoing) = commands::execute_routine(
                        routine_store.clone(),
                        None,
                        &map_state,
                        &logger,
                    ).await;

                    if let Some(next_routine) = next_routine_opt {
                         let _ = routine_tx.send(RoutineMessage::Override(next_routine)).await;
                    }
                    
                    for out_msg in outgoing {
                        let _ = output_tx.send(Message::Text(out_msg.into())).await;
                    }
                }
                RoutineMessage::GameMessage(game_msg) => {
                   let routine_store = current_routine.lock().await;
                   let (next_routine_opt, outgoing) = commands::execute_routine(
                        routine_store.clone(),
                        Some(game_msg),
                        &map_state,
                        &logger,
                    ).await;

                    if let Some(next_routine) = next_routine_opt {
                         let _ = routine_tx.send(RoutineMessage::Override(next_routine)).await;
                    }
                    
                    for out_msg in outgoing {
                        let _ = output_tx.send(Message::Text(out_msg.into())).await;
                    }
                }
            }
        }
    });
}

async fn run_repl(
    mut rl: Readline,
    logger: Logger,
    output_tx: mpsc::Sender<Message>,
    routine_tx: mpsc::Sender<RoutineMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match rl.readline().await {
            Ok(ReadlineEvent::Line(line)) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                rl.add_history_entry(line.to_string());

                if line.starts_with('/') {
                     if let Some(new_routine) = commands::handle_repl_command(line, &logger).await {
                         let _ = routine_tx.send(RoutineMessage::Override(new_routine)).await;
                     }
                } else {
                    let _ = output_tx.send(Message::Text(line.into())).await;
                }
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

use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_serial::SerialPortBuilderExt;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(name = "rust-serial-bridge")]
#[command(about = "Bridges Arduino serial data to a WebSocket server")]
struct Args {
    /// Serial port (e.g. COM3 on Windows, /dev/ttyUSB0 on Linux)
    #[arg(short, long, default_value = "COM3")]
    port: String,

    /// Baud rate for the serial connection
    #[arg(short, long, default_value_t = 9600)]
    baud: u32,

    /// WebSocket server host
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// WebSocket server port
    #[arg(short = 'w', long, default_value_t = 8080)]
    ws_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // Broadcast channel: serial data → all connected WebSocket clients
    let (tx, _rx) = broadcast::channel::<String>(256);

    // ── Spawn the serial reader task ──
    let serial_tx = tx.clone();
    let port_name = args.port.clone();
    let baud_rate = args.baud;

    tokio::spawn(async move {
        loop {
            info!("Opening serial port {} at {} baud", port_name, baud_rate);

            match tokio_serial::new(&port_name, baud_rate).open_native_async() {
                Ok(mut serial) => {
                    info!("Serial port opened successfully");

                    let mut buf = vec![0u8; 1024];
                    let mut line_buf = String::new();

                    loop {
                        match serial.read(&mut buf).await {
                            Ok(0) => {
                                warn!("Serial port returned 0 bytes, reconnecting…");
                                break;
                            }
                            Ok(n) => {
                                let chunk = String::from_utf8_lossy(&buf[..n]);
                                line_buf.push_str(&chunk);

                                // Emit complete lines
                                while let Some(pos) = line_buf.find('\n') {
                                    let line = line_buf[..pos].trim().to_string();
                                    line_buf = line_buf[pos + 1..].to_string();

                                    if !line.is_empty() {
                                        let parts: Vec<&str> = line.split(',').collect();
                                        let payload = if parts.len() >= 8 {
                                            let vals: Result<Vec<f64>, _> = parts.iter().map(|s| s.parse::<f64>()).collect();
                                            if let Ok(vals) = vals {
                                                serde_json::json!({
                                                    "fingers": {
                                                        "pinky": vals[0],
                                                        "ring": vals[1],
                                                        "middle": vals[2],
                                                        "index": vals[3],
                                                        "thumb": vals[4]
                                                    },
                                                    "orientation": {
                                                        "roll": vals[5],
                                                        "pitch": vals[6],
                                                        "yaw": vals[7]
                                                    }
                                                })
                                            } else {
                                                serde_json::json!({
                                                    "type": "serial",
                                                    "data": line,
                                                    "timestamp": chrono_now_ms()
                                                })
                                            }
                                        } else {
                                            serde_json::json!({
                                                "type": "serial",
                                                "data": line,
                                                "timestamp": chrono_now_ms()
                                            })
                                        };
                                        let msg = payload.to_string();
                                        // Ignore send errors (no subscribers yet)
                                        let _ = serial_tx.send(msg);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Serial read error: {e}");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to open serial port: {e}");
                }
            }

            // Wait before attempting to reconnect
            info!("Retrying serial connection in 2 seconds…");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    // ── WebSocket server ──
    let addr = format!("{}:{}", args.host, args.ws_port);
    let listener = TcpListener::bind(&addr).await?;
    info!("WebSocket server listening on ws://{addr}");

    loop {
        let (stream, peer) = listener.accept().await?;
        info!("New WebSocket connection from {peer}");

        let rx = tx.subscribe();
        tokio::spawn(handle_ws_client(stream, rx));
    }
}

async fn handle_ws_client(
    stream: tokio::net::TcpStream,
    mut rx: broadcast::Receiver<String>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed: {e}");
            return;
        }
    };

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Forward broadcast messages to the WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if ws_tx.send(Message::Text(msg)).await.is_err() {
                break; // client disconnected
            }
        }
    });

    // Drain incoming WebSocket messages (keep-alive / pong handling)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(_msg)) = ws_rx.next().await {
            // We don't expect commands from the browser, just drain.
        }
    });

    // When either task exits, the connection is done
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!("WebSocket client disconnected");
}

/// Returns milliseconds since UNIX epoch (no extra crate needed).
fn chrono_now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

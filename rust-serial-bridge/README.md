# Rust Serial Bridge

Reads data from an Arduino serial port and broadcasts it to browser clients over WebSocket.

## Prerequisites

1. Install Rust: https://rustup.rs
2. Plug in your Arduino and note the COM port (e.g. `COM3` on Windows).

## Build & Run

```bash
cargo build --release
cargo run -- --port COM3 --baud 9600 --ws-port 8080
```

### CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `-p, --port` | `COM3` | Serial port name |
| `-b, --baud` | `9600` | Baud rate |
| `--host` | `127.0.0.1` | WebSocket bind address |
| `-w, --ws-port` | `8080` | WebSocket port |

## Connect from the Browser

```js
const ws = new WebSocket("ws://localhost:8080");
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log(data);
  // { type: "serial", data: "23.5", timestamp: 1709830000000 }
};
```

Each line the Arduino prints via `Serial.println()` is sent as a JSON message.

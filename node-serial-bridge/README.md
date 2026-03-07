# Node.js Serial Bridge

Reads data from an Arduino serial port and broadcasts it to browser clients over WebSocket.

## Prerequisites

1. Node.js 18+ installed
2. Plug in your Arduino and note the COM port (e.g. `COM3` on Windows).

## Install & Run

```bash
npm install
npm start
# or with custom settings:
node index.js COM3 9600 8080
```

### Environment Variables (alternative)

| Variable | Default | Description |
|----------|---------|-------------|
| `SERIAL_PORT` | `COM3` | Serial port name |
| `BAUD_RATE` | `9600` | Baud rate |
| `WS_PORT` | `8080` | WebSocket port |

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

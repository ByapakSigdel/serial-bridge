const { SerialPort } = require("serialport");
const { ReadlineParser } = require("serialport");
const { WebSocketServer } = require("ws");

// ── Configuration (override via env vars or CLI args) ──
const SERIAL_PORT = process.env.SERIAL_PORT || process.argv[2] || "COM3";
const BAUD_RATE = parseInt(process.env.BAUD_RATE || process.argv[3] || "9600", 10);
const WS_PORT = parseInt(process.env.WS_PORT || process.argv[4] || "8080", 10);

// ── WebSocket server ──
const wss = new WebSocketServer({ port: WS_PORT });
console.log(`WebSocket server listening on ws://127.0.0.1:${WS_PORT}`);

const clients = new Set();

wss.on("connection", (ws, req) => {
  const peer = req.socket.remoteAddress;
  console.log(`New WebSocket connection from ${peer}`);
  clients.add(ws);

  ws.on("close", () => {
    console.log(`Client ${peer} disconnected`);
    clients.delete(ws);
  });

  ws.on("error", (err) => {
    console.error(`WebSocket error from ${peer}:`, err.message);
    clients.delete(ws);
  });
});

function broadcast(message) {
  const payload = JSON.stringify(message);
  for (const client of clients) {
    if (client.readyState === client.OPEN) {
      client.send(payload);
    }
  }
}

// ── Serial port reader with auto-reconnect ──
function connectSerial() {
  console.log(`Opening serial port ${SERIAL_PORT} at ${BAUD_RATE} baud`);

  const port = new SerialPort({
    path: SERIAL_PORT,
    baudRate: BAUD_RATE,
    autoOpen: false,
  });

  const parser = port.pipe(new ReadlineParser({ delimiter: "\n" }));

  parser.on("data", (line) => {
    const trimmed = line.trim();
    if (trimmed.length === 0) return;

    const message = {
      type: "serial",
      data: trimmed,
      timestamp: Date.now(),
    };

    broadcast(message);
  });

  port.on("open", () => {
    console.log("Serial port opened successfully");
  });

  port.on("error", (err) => {
    console.error("Serial port error:", err.message);
  });

  port.on("close", () => {
    console.log("Serial port closed, retrying in 2 seconds…");
    setTimeout(connectSerial, 2000);
  });

  port.open((err) => {
    if (err) {
      console.error("Failed to open serial port:", err.message);
      console.log("Retrying in 2 seconds…");
      setTimeout(connectSerial, 2000);
    }
  });
}

connectSerial();

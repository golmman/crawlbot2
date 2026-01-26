const WebSocket = require("ws");
const readline = require("readline");
const zlib = require("zlib");
const JSONStream = require("JSONStream");

const map = require("./map.js");
const { createMessage } = require("./commands.js");

const messageHook = { callback: null };

const url = "ws://127.0.0.1:8080/socket";
const ws = new WebSocket(url, {
  headers: { Origin: "http://127.0.0.1:8080" },
});

const parser = JSONStream.parse("*");

const decompressor = zlib.createInflateRaw();


const currentCommand = null;

function processMessage(inMsg) {
  if (!currentCommand) {
    return;
  }

  const { message, nextCommand } = currentCommand(inMsg);

  ws.send(JSON.stringify({ msg: "pong" }));
  currentCommand = nextCommand;
}

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  prompt: `${new Date().toISOString()} DCSS    > `,
});

parser.on("data", (messages) => {
  try {
    console.log(
      `\n${new Date().toISOString()} [Server]:`,
      JSON.stringify(messages),
    );

    const mapMessage = messages.find((msg) => msg.msg === "map");
    const pingMessage = messages.find((msg) => msg.msg === "ping");

    if (mapMessage) {
      map.updateMap(mapMessage.cells);
      map.printMap();
    }

    if (pingMessage) {
      setTimeout(() => {
        ws.send(JSON.stringify({ msg: "pong" }));
        console.log(
          `\n${new Date().toISOString()} [Client]: pong message sent`,
        );
        rl.prompt();
      }, 5000);
    }

    rl.prompt();
  } catch (err) {
    console.error("Error handling parsed JSON:", err);
  }
});

parser.on("error", (err) => {
  console.error("JSON stream parse error:", err);
});

decompressor.pipe(parser);

ws.on("open", () => {
  console.log("Connected. Forcing Manual Decompression...");
  rl.prompt();
});

ws.on("message", (data) => {
  // WebSockets with per-message deflate append 00 00 ff ff to every frame
  // Manual zlib streams need this to know a block has ended.
  const syncBuffer = Buffer.from([0x00, 0x00, 0xff, 0xff]);
  decompressor.write(data);
  decompressor.write(syncBuffer);
});

rl.on("line", (line) => {
  if (line.startsWith("/")) {
    const message = ""
    if (message.length > 0) ws.send(message);
  } else if (line.trim()) {
    ws.send(line.trim());
  }

  rl.prompt();
});

ws.on("close", () => process.exit());

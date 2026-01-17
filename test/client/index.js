const WebSocket = require('ws');
const readline = require('readline');
const zlib = require('zlib');

const url = 'ws://127.0.0.1:8080/socket';
const ws = new WebSocket(url, {
    headers: { 'Origin': 'http://127.0.0.1:8080' }
});

// Create a persistent decompressor to handle "Context Takeover"
const decompressor = zlib.createInflateRaw();

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    prompt: 'DCSS> '
});

// Handle the decompressed data coming out of the zlib stream
decompressor.on('data', (chunk) => {
    const messageString = chunk.toString();
    try {
        const json = JSON.parse(messageString);
        // Filter out the noisy map updates
        if (!['vgrdc', 'map', 'viewport'].includes(json.msg)) {
            console.log(`\n[Server]:`, JSON.stringify(json, null, 2));
            rl.prompt();
        } else {
            process.stdout.write('.'); // Heartbeat for map data
        }
    } catch (e) {
        console.log(`\n[Decompressed Text (Partial?)]: ${messageString}`);
    }
});

ws.on('open', () => {
    console.log('✅ Connected. Forcing Manual Decompression...');
    ws.send(JSON.stringify({ msg: "client_id", id: "web" }));
    rl.prompt();
});

ws.on('message', (data) => {
    // WebSockets with per-message deflate append 00 00 ff ff to every frame
    // Manual zlib streams need this to know a block has ended.
    const syncBuffer = Buffer.from([0x00, 0x00, 0xff, 0xff]);
    decompressor.write(data);
    decompressor.write(syncBuffer);
});

rl.on('line', (line) => {
    if (line.trim()) ws.send(line.trim());
    rl.prompt();
});

ws.on('close', () => process.exit());

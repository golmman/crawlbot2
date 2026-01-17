const WebSocket = require('ws');
const readline = require('readline');

// Use the IPv4 address we verified earlier
const url = 'ws://127.0.0.1:8080/socket';
const options = {
    headers: { 'Origin': 'http://127.0.0.1:8080' }
};

const ws = new WebSocket(url, options);

// Set up the terminal interface
const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    prompt: 'DCSS> '
});

ws.on('open', () => {
    console.log('✅ Connected to DCSS Webtiles');
    console.log('Type your JSON and press Enter. (e.g., {"msg":"play","game_id":"crawl-0.31"})\n');
    rl.prompt();
});

ws.on('message', (data) => {
    try {
        const msg = JSON.parse(data.toString());
        
        // Filter out the massive tile data (vgrdc) to keep the terminal readable
        if (msg.msg !== 'vgrdc' && msg.msg !== 'map') {
            process.stdout.write(`\r\x1b[K`); // Clear current line
            console.log(`\n[Server]:`, JSON.stringify(msg, null, 2));
            rl.prompt();
        } else {
            // Optional: Just print a tiny indicator that map data was received
            process.stdout.write('.'); 
        }

        // Auto-respond to pings to prevent timeout
        if (msg.msg === 'ping') {
            ws.send(JSON.stringify({ msg: 'pong' }));
        }
    } catch (e) {
        console.log('\n[Raw Data]:', data.toString());
    }
});

rl.on('line', (line) => {
    const input = line.trim();
    if (input) {
        try {
            // Validate it is JSON before sending
            JSON.parse(input); 
            ws.send(input);
        } catch (e) {
            console.log('❌ Invalid JSON. Format: {"msg":"...", ...}');
        }
    }
    rl.prompt();
});

ws.on('error', (err) => console.error('\n❌ WebSocket Error:', err.message));
ws.on('close', () => {
    console.log('\n🔌 Connection closed');
    process.exit();
});

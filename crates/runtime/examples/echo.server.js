// WebSocket Echo Server for Node.js 20+
// Usage: node echo.server.js

import { WebSocketServer } from 'ws';

console.log(1111);

const PORT = 9001;

const wss = new WebSocketServer({ port: PORT });
console.log(2222);

console.log(`WebSocket Echo Server listening on ws://localhost:${PORT}`);

wss.on('connection', (ws, req) => {
    const clientIp = req.socket.remoteAddress;
    console.log(`[Connection] New client connected from ${clientIp}`);

    ws.on('message', (data, isBinary) => {
        console.log(`[Message] Received ${isBinary ? 'binary' : 'text'}: ${isBinary ? `${data.length} bytes` : data}`);
        
        // Echo back the message
        ws.send(data, { binary: isBinary });
    });

    ws.on('close', (code, reason) => {
        console.log(`[Close] Client disconnected. Code: ${code}, Reason: ${reason || 'none'}`);
    });

    ws.on('error', (error) => {
        console.error(`[Error] WebSocket error:`, error);
    });

    ws.on('ping', (data) => {
        console.log(`[Ping] Received ping: ${data}`);
    });

    ws.on('pong', (data) => {
        console.log(`[Pong] Received pong: ${data}`);
    });

    // Send welcome message
    ws.send('Welcome to Echo Server!');
});

wss.on('error', (error) => {
    console.error('[Server Error]', error);
});

// Graceful shutdown
process.on('SIGINT', () => {
    console.log('\n[Shutdown] Closing server...');
    wss.close(() => {
        console.log('[Shutdown] Server closed');
        process.exit(0);
    });
});

const http = require('http');

const PORT = 8000;

const server = http.createServer((req, res) => {
    res.setHeader('Content-Type', 'text/plain');
    res.end('ok');
});
server.on('clientError', (err, socket) => {
    socket.end('HTTP/1.1 400 Bad Request\r\n\r\n');
});
server.listen(PORT, () => {
    console.log(`Server listening on port ${PORT}`);
});

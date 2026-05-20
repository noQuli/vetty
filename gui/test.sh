#!/bin/bash
node -e "const http = require('http'); const server = http.createServer(); server.on('error', (e) => console.log('ERROR:', e.message)); server.listen(5173, () => { console.log('SUCCESS: listening on 5173'); process.exit(0); });" > ../node_out.txt 2>&1

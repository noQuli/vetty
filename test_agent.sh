#!/bin/bash
FIFO="/tmp/test_fifo_$$"
mkfifo "$FIFO"

cargo run -p vetty-agent < "$FIFO" &
AGENT_PID=$!

strace -f -tt -T -e trace=file,network,process -o "$FIFO" -- ls
wait "$AGENT_PID"

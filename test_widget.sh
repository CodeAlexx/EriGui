#!/bin/bash
cargo run --example widget_gallery 2>&1 &
PID=$!
sleep 2
kill $PID 2>/dev/null
#!/bin/sh
cd server
cargo run
cd ..
npm run dev -- --host

cargo web build --release --target wasm32-unknown-unknown

cp target/wasm32-unknown-emscripten/release/saltybet.js static/saltybet.js
cp target/wasm32-unknown-emscripten/release/saltybet.wasm static/saltybet.wasm

cp target/wasm32-unknown-emscripten/release/twitch_chat.js static/twitch_chat.js
cp target/wasm32-unknown-emscripten/release/twitch_chat.wasm static/twitch_chat.wasm

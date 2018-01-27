cargo web build --release --target asmjs-unknown-emscripten --use-system-emscripten

echo "var Module={};" | cat - target/asmjs-unknown-emscripten/release/saltybet.js > static/saltybet.js
echo "var Module={};" | cat - target/asmjs-unknown-emscripten/release/twitch_chat.js > static/twitch_chat.js

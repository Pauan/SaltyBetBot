const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const dist = path.resolve(__dirname, "dist");

module.exports = {
  mode: "production",
  stats: "errors-warnings",
  entry: {
    background: "./js/background.js",
    chart: "./js/chart.js",
    popup: "./js/popup.js",
    records: "./js/records.js",
    saltybet: "./js/saltybet.js",
    twitch_chat: "./js/twitch_chat.js",
  },
  output: {
    path: dist,
    filename: "[name].js"
  },
  plugins: [
    new CopyPlugin([
      path.resolve(__dirname, "static")
    ]),

    new WasmPackPlugin({
      crateDirectory: path.join(__dirname, "src", "background"),
      extraArgs: "--out-name background --out-dir ../../pkg"
    }),

    new WasmPackPlugin({
      crateDirectory: path.join(__dirname, "src", "chart"),
      extraArgs: "--out-name chart --out-dir ../../pkg"
    }),

    new WasmPackPlugin({
      crateDirectory: path.join(__dirname, "src", "popup"),
      extraArgs: "--out-name popup --out-dir ../../pkg"
    }),

    new WasmPackPlugin({
      crateDirectory: path.join(__dirname, "src", "records"),
      extraArgs: "--out-name records --out-dir ../../pkg"
    }),

    new WasmPackPlugin({
      crateDirectory: path.join(__dirname, "src", "saltybet"),
      extraArgs: "--out-name saltybet --out-dir ../../pkg"
    }),

    new WasmPackPlugin({
      crateDirectory: path.join(__dirname, "src", "twitch_chat"),
      extraArgs: "--out-name twitch_chat --out-dir ../../pkg"
    }),
  ]
};

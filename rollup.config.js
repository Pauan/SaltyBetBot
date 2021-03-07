import rust from "@wasm-tool/rollup-plugin-rust";
import { terser } from "rollup-plugin-terser";

export default {
    input: {
        background: "src/background/Cargo.toml",
        chart: "src/chart/Cargo.toml",
        popup: "src/popup/Cargo.toml",
        records: "src/records/Cargo.toml",
        saltybet: "src/saltybet/Cargo.toml",
        twitch_chat: "src/twitch_chat/Cargo.toml",
    },
    output: {
        dir: "static/js",
        format: "esm",
        sourcemap: true,
        assetFileNames: "assets/[name][extname]",
    },
    plugins: [
        rust({
            serverPath: "js/",
            importHook: function (path) {
                return "chrome.runtime.getURL(" + JSON.stringify(path) + ")";
            },
        }),
        //terser(),
    ],
};

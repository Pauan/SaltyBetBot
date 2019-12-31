const $fs = require("fs");
const $path = require("path");
const $child = require("child_process");
const $toml = require("toml");
const $rimraf = require("rimraf");
const { createFilter } = require("rollup-pluginutils");


function rm(path) {
    return new Promise(function (resolve, reject) {
        $rimraf(path, { glob: false }, function (err) {
            if (err) {
                reject(err);

            } else {
                resolve();
            }
        });
    });
}

function read(path) {
    return new Promise(function (resolve, reject) {
        $fs.readFile(path, function (err, file) {
            if (err) {
                reject(err);

            } else {
                resolve(file);
            }
        });
    });
}

function wait(p) {
    return new Promise((resolve, reject) => {
        p.on("close", (code) => {
            if (code === 0) {
                resolve();

            } else {
                reject(new Error("Command `" + p.spawnargs.join(" ") + "` failed with error code: " + code));
            }
        });

        p.on("error", reject);
    });
}


const state = {
    locked: false,
    pending: [],
};

async function lock(f) {
    if (state.locked) {
        await new Promise(function (resolve, reject) {
            state.pending.push(resolve);
        });

        if (state.locked) {
            throw new Error("Invalid lock state");
        }
    }

    state.locked = true;

    try {
        return await f();

    } finally {
        state.locked = false;

        if (state.pending.length !== 0) {
            const resolve = state.pending.shift();
            // Wake up pending task
            resolve();
        }
    }
}


async function build(cx, id, options) {
    const toml = $toml.parse(await read(id));

    const name = toml.package.name;

    const dir = $path.dirname(id);

    // TODO use some logic to find the target dir
    const out_dir = $path.resolve($path.join("target", "wasm-pack", name));

    await rm(out_dir);

    const args = [
        // TODO adjust based on Webpack's error report level
        //"--log-level", "error",
        "build",
        "--out-dir", out_dir,
        "--out-name", "index",
        "--target", "web",
        "--no-typescript",
        (options.debug ? "--dev" : "--release")
    ];

    try {
        await lock(async function () {
            // TODO make sure to use the npm installed binary for wasm-pack
            await wait($child.spawn("wasm-pack", args, { cwd: dir, stdio: "inherit" }));
        });

    // TODO print the full error in verbose mode
    } catch (e) {
        throw new Error("");
    }

    const wasm = await read($path.join(out_dir, "index_bg.wasm"));

    // TODO use the [name] somehow
    const wasm_name = name + ".wasm";

    cx.emitFile({
        type: "asset",
        source: wasm,
        fileName: wasm_name
    });

    // TODO better way to generate the path
    const import_path = JSON.stringify("./" + $path.relative(dir, $path.join(out_dir, "index.js")));

    const import_wasm = (options.importHook ? options.importHook(wasm_name) : JSON.stringify(wasm_name));

    return `
        import init from ${import_path};

        init(${import_wasm}).catch(console.error);
    `;
}


module.exports = function rust(options = {}) {
    const filter = createFilter(options.include, options.exclude);

    if (options.debug == null) {
        options.debug = false;
    }

    return {
        name: "rust",

        load(id) {
            if ($path.basename(id) === "Cargo.toml" && filter(id)) {
                return build(this, id, options);

            } else {
                return null;
            }
        },

        // TODO hacky, improve this
        resolveImportMeta(property, { moduleId }) {
            if (property === "url") {
                return "\"\"";
            }

            return null;
        },
    };
};

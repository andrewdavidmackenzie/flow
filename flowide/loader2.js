import {default as init} from "./pkg/flowide.js";

console.log("loading flowide.js for WASM module");

async function run() {
    await init();
}

run();
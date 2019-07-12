const ffi = require('ffi-napi');
const ref_napi = require("ref-napi");
const path = require('path');
const lib_path = './target/debug/libflow';
const strPtr = ref_napi.refType(ref_napi.types.CString);
let flow = ffi.Library(path.join(__dirname, lib_path), {
    version: [strPtr, []]
});

document.getElementById('flowlibc').innerText = flow.version();
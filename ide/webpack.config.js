const path = require("path");
const dist = path.resolve(__dirname, "dist");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

module.exports = {
    mode: 'development',
    entry: {
        index: "./js/index.js"
    },
    output: {
        path: dist,
        filename: "[name].js"
    },
    devServer: {
        contentBase: dist,
    },
    plugins: [
        new CopyPlugin([
            path.resolve(__dirname, "static"),
            path.resolve(__dirname, "js"),
        ]),

        new WasmPackPlugin({
            crateDirectory: __dirname,
            extraArgs: "--out-name index"
        }),
    ]
};
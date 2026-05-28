# Running Average

This example implements the running average calculation from the Lucid dataflow
programming language book ("Lucid, the Dataflow Programming Language" by Wadge
and Ashcroft, 1985, page 47).

## The Lucid Program

```
next s/n
where
    s = 0 fby s + x;
    n = 0 fby n + 1;
end
```

Where:
- `x` is a stream of input numbers (read from stdin)
- `s` is the running sum, initialized to 0
- `n` is the running count, initialized to 0
- The output is `s/n` — the running average after each input

## Flow Implementation

The flow uses only `flowstdlib` functions:
- `add` with loopback implements `s = 0 fby s + x`
- `count` with loopback implements `n = 0 fby n + 1`
- `divide` computes `s / n`

No custom Rust/WASM functions are needed.

## Usage

Pipe numbers (one per line) to stdin:
```
echo "10\n20\n30" | flowrcli --native manifest.json
```

Output will be the running average after each input:
```
10
15
20
```

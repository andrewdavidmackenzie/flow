# Weather Station — Streaming Sensor Data Processing

A streaming example that processes temperature readings one at a time,
computing running statistics in parallel using only flowstdlib primitives.

Each new reading triggers — in parallel:
- **Running minimum** (`lib://flowstdlib/data/min`)
- **Running maximum** (`lib://flowstdlib/data/max`)
- **Running mean** (`lib://flowstdlib/data/avg` + `lib://flowstdlib/math/divide`)
- **Reading count** (`lib://flowstdlib/data/count`)

Each stat is labeled with `lib://flowstdlib/data/append` and sent to stdout
independently. The output order between stats is non-deterministic — this
demonstrates the parallelism inherent in the dataflow model.

## Running interactively

Type temperature values (one per line) and see stats update after each:

```
flowrcli flowr/examples/weather-station
```

## Running with the data generator

Stream pseudo-random temperature data continuously:

```
./flowr/examples/weather-station/generate_data.sh | flowrcli flowr/examples/weather-station
```

## Test data

The included `test.stdin` contains 96 readings (24 hours at 15-minute
intervals) simulating a Mediterranean summer day with a cold front arriving
in the late afternoon.

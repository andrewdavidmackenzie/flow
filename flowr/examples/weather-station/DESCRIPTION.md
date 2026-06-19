# Weather Station — Streaming Sensor Data Processing

A streaming example that processes temperature readings one at a time,
maintaining a sliding window of the last 24 values (6 hours at 15-minute
intervals).

Each new reading triggers:
- **Sliding window update**: the window accumulates up to 24 values, then
  drops the oldest on each new reading
- **Statistical analysis**: min, max, mean over the current window
- **Anomaly detection**: flags sudden temperature changes (>3°C between
  consecutive readings) within the window
- **Temperature distribution**: histogram image updated with each reading

## Running interactively

Type temperature values (one per line) and see analysis after each:

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

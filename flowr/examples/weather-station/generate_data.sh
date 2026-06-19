#!/bin/bash
# Stream pseudo-random temperature readings forever (one value per line).
# Uses the current time of day for a realistic diurnal cycle.
# Usage: ./generate_data.sh | flowrcli flowr/examples/weather-station
#        ./generate_data.sh 5    # one reading every 5 seconds (default: 2)

INTERVAL="${1:-2}"

while true; do
    python3 -c "
import math, random, time
hour = (time.time() % 86400) / 3600
base = 23.5 + 8.5 * math.sin(math.pi * (hour - 5) / 12 - math.pi / 2)
noise = random.gauss(0, 0.8)
print(round(base + noise, 1))
"
    sleep "$INTERVAL"
done

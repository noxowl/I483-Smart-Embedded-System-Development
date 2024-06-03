#!/bin/sh

# Run the application
cargo run listen 150.65.230.59 i483/sensors/s2420010/SCD41/temperature i483/sensors/s2420010/SCD41/co2 i483/sensors/s2420010/SCD41/humidity i483/sensors/s2420010/BMP180/temperature i483/sensors/s2420010/BMP180/air_pressure

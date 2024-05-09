"""I483 Assignment 01 (b): SCD41 CO2 Sensor
Author: Suyeong Rhie

The Conditions of Assignment:
- The SCD41 sensor should be connected to the ESP32 using I2C.
- The ESP32 should be programmed using MicroPython.
- The ESP32 should read the sensor value from the SCD41 sensor every second (max 15sec).
- The ESP32 should print the sensor value to the console.
"""
from machine import Pin, I2C
from scd41 import SCD41
import time


bus = I2C(0, scl=Pin(33, pull=Pin.PULL_UP), sda=Pin(32, pull=Pin.PULL_UP), freq=400000, timeout=1000000)
scan_res = bus.scan()
print([hex(x) for x in scan_res])

scd41 = SCD41(bus)
TIMEOUT = 15

scd41.periodic_measurements(True)
while TIMEOUT > 0:
    scd41.print_current_measurement()
    TIMEOUT -= 1
    time.sleep_ms(1000)
scd41.periodic_measurements(False)

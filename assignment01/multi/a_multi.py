"""I483 Assignment 01 2-(a): BMP180 and SCD41 Sensors
Author: Suyeong Rhie

The Conditions of Assignment:
- The BMP180 sensor should be connected to the ESP32 using I2C.
- The SCD41 sensor should be connected to the ESP32 using I2C.
- The ESP32 should be programmed using MicroPython.
- The ESP32 should read the sensor values from the BMP180 and SCD41 sensors every second (max 15sec).
- The ESP32 should print the sensor values to the console.
"""

from machine import Pin, I2C
from scd41 import SCD41
from bmp180 import BMP180
import time


bus = I2C(0, scl=Pin(33, pull=Pin.PULL_UP), sda=Pin(32, pull=Pin.PULL_UP), freq=400000, timeout=1000000)
scan_res = bus.scan()
print([hex(x) for x in scan_res])

_scd41 = SCD41(bus)
_bmp180 = BMP180(bus)

TIMEOUT = 15

_scd41.periodic_measurements(True)
while TIMEOUT > 0:
    print(f"Measuring... {TIMEOUT}s left")
    _bmp180.print_current_measurement()
    _scd41.print_current_measurement()
    print("--------------------\n")
    TIMEOUT -= 1
    time.sleep_ms(1000)
_scd41.periodic_measurements(False)

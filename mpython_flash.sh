#!/bin/bash

esptool.py --chip esp32 --port /dev/tty.usbserial-0001 erase_flash
esptool.py --chip esp32 --port /dev/tty.usbserial-0001 write_flash -z 0x1000 ./ESP32_GENERIC-20240222-v1.22.2.bin

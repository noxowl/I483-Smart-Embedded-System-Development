#!/bin/bash

if ! command -v ampy &> /dev/null
then
    echo "ampy could not be found"
    exit 1
fi

ampy --port /dev/tty.usbserial-0001 put ../assignment01/scd41/mpython/scd41.py scd41.py
ampy --port /dev/tty.usbserial-0001 put ../practice01/practice_bmp180.py bmp180.py
ampy --port /dev/tty.usbserial-0001 put ./config.json config.json
ampy --port /dev/tty.usbserial-0001 put ./mqtt_pub.py main.py

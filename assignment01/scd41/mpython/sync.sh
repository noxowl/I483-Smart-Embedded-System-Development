#!/bin/bash

if ! command -v ampy &> /dev/null
then
    echo "ampy could not be found"
    exit 1
fi

ampy --port /dev/tty.usbserial-0001 put ./scd41.py scd41.py
ampy --port /dev/tty.usbserial-0001 put ./a_scd41.py main.py

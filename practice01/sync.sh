#!/bin/bash

if ! command -v ampy &> /dev/null
then
    echo "ampy could not be found"
    exit 1
fi

ampy --port /dev/tty.usbserial-0001 put ./practice_bmp180.py main.py

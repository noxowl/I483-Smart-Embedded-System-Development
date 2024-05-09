#!/bin/bash

if ! command -v ampy &> /dev/null
then
    echo "ampy could not be found"
    exit 1
fi

ampy --port /dev/tty.usbserial-0001 put blinky.py main.py

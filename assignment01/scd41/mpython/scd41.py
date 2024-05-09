"""
SCD41 - CO2, Temperature and Humidity Sensor
Author: Suyeong Rhie

This module contains the SCD41 class which is used to interact with the Sensirion SCD41 CO2 sensor.

The SCD41 class provides the following functionality:
- Start and stop periodic measurements
- Check if data is ready to be read from the sensor
- Get a measurement from the sensor
"""
from machine import I2C
from micropython import const
import struct
import time

SCD41_ADDRESS = const(0x62)
SCD4x_STOP_PERIODIC_MEASUREMENT = const(0x3F86)
SCD4x_START_PERIODIC_MEASUREMENT = const(0x21B1)
SCD4x_READ_MEASUREMENT = const(0xEC05)
SCD4x_GET_DATA_READY_STATUS = const(0xE4B8)
SCD4x_SINGLE_SHOT_MEASUREMENT = const(0x219D)
SCD4x_SINGLE_SHOT_MEASUREMENT_RHT = const(0x2196)
SCD4x_WAKE_UP = const(0x36F6)
SCD4x_START_LOW_POWER_PERIODIC_MEASUREMENT = const(0x21AC)
SCD4x_SELF_TEST = const(0x3639)

class SCD41():
    def __init__(self, bus: I2C, do_self_test: bool = False):
        self._message_buffer = bytearray(18)
        self._command_buffer = bytearray(2)
        self._crc_buffer = bytearray(2)
        self.bus: I2C = bus
        self.address = SCD41_ADDRESS

        self._temperature = None
        self._humidity = None
        self._co2 = None

        print(f"Init SCD41(Addr: {self.address}) from I2C Bus {bus}")

        self.wake_up()
        if do_self_test:
            self.self_test()
        self.stop_periodic_measurements()
    
    def is_data_ready_print(self) -> None:
        print("Is data ready? {}.".format("yes" if self._is_data_ready() else "no"))

    def _send_command(self, command, delay: int = 1) -> None:
        self._command_buffer[0] = (command >> 8) & 0xFF
        self._command_buffer[1] = command & 0xFF
        try:
            self.bus.writeto(self.address, self._command_buffer)
        except OSError as err:
            # [Errno 19] ENODEV: poor contacts (Check the wiring) 
            raise RuntimeError(f"Can't communicate via I2C: {err}")
        time.sleep_ms(delay)

    def _receive_reply(self, buffer, length) -> None:
        self.bus.readfrom_into(self.address, buffer)
        if not self._is_crc_correct(buffer[0:length]):
            raise RuntimeError("CRC Error")
    
    def _is_crc_correct(self, data: bytearray) -> bool:
        for i in range(0, len(data), 3):
            self._crc_buffer[0] = data[i]
            self._crc_buffer[1] = data[i + 1]
            if self._sensrion_crc8(self._crc_buffer) != data[i + 2]:
                return False
        return True

    @staticmethod
    def _sensrion_crc8(data: bytearray) -> int:
        crc = 0xFF
        for byte in data:
            crc ^= byte
            for _ in range(8):
                if crc & 0x80:
                    crc = (crc << 1) ^ 0x31
                else:
                    crc <<= 1
        return crc & 0xFF

    def start_periodic_measurements(self) -> None:
        print("Starting periodic measurements")
        self._send_command(SCD4x_START_PERIODIC_MEASUREMENT)
    
    def stop_periodic_measurements(self) -> None:
        print("Stopping periodic measurements")
        self._send_command(SCD4x_STOP_PERIODIC_MEASUREMENT, 500)

    def periodic_measurements(self, start: bool) -> None:
        if start:
            self.start_periodic_measurements()
        else:
            self.stop_periodic_measurements()

    def _is_data_ready(self) -> bool:
        """Check if data is ready to be read from the sensor.
        Least significant 11 bits are 0 → data not ready
        When data is not ready: bytearray(b'\x00\x00\x81\x00\x00\x81\x00\x00\x81\x00\x00\x81\x00\x00\x81\x00\x00\x81')
        """
        self._send_command(SCD4x_GET_DATA_READY_STATUS)
        self._receive_reply(self._message_buffer, 3)

        # return (self._message_buffer[0] << 8 | self._message_buffer[1]) & 0x1FFF == 0
        return not ((self._message_buffer[0] & 0x07 == 0) and (self._message_buffer[1] == 0))

    def get_measurement(self) -> None:
        print("Getting measurement")
        if not self._is_data_ready():
            print("Data is not ready yet")
            return
        self.read_measurement()

        self._co2 = (self._message_buffer[0] << 8) | self._message_buffer[1]
        raw_temperature = (self._message_buffer[3] << 8) | self._message_buffer[4]
        raw_humidity = (self._message_buffer[6] << 8) | self._message_buffer[7]

        self._temperature = -45 + 175 * (raw_temperature / 65536)
        self._humidity = 100 * (raw_humidity / 65536)

    def wake_up(self) -> None:
        """
        This command is used to wake up the sensor from the idle mode. The sensor will return to the measurement mode.
        """
        print("Waking up")
        self._send_command(SCD4x_WAKE_UP, 50)
    
    def single_shot_measurement(self) -> None:
        """
        This command triggers a single-shot measurement and returns to the idle mode after the measurement is finished.
        """
        print("Starting single shot measurement")
        self._send_command(SCD4x_SINGLE_SHOT_MEASUREMENT, 5000)

    def single_shot_measurement_rht(self) -> None:
        print("Starting single shot measurement RHT")
        self._send_command(SCD4x_SINGLE_SHOT_MEASUREMENT_RHT, 50)

    def self_test(self) -> None:
        """
        This command triggers a self-test of the sensor. The sensor will return to the idle mode after the self-test is finished.
        """
        print("Starting self-test")
        self._send_command(SCD4x_SELF_TEST, 10000)
        self._receive_reply(self._message_buffer, 3)
        malfunc = self._message_buffer[0] << 8 | self._message_buffer[1]
        if malfunc != 0:
            raise RuntimeError(f"Self-test failed: {malfunc}")
        print("Self-test passed")
        
    def read_measurement(self) -> None:
        self._send_command(SCD4x_READ_MEASUREMENT)
        self._receive_reply(self._message_buffer, 9)

    @property
    def co2(self) -> int:
        return self._co2
    
    @property
    def temperature(self) -> float:
        return self._temperature
    
    @property
    def humidity(self) -> float:
        return self._humidity

    def print_current_measurement(self) -> None:
        if self._is_data_ready():
            self.get_measurement()
        else:
            print("Data is not ready yet. Return the cached data.")
        print(f"SCD41: CO2_{self.co2}(ppm), Temperature_{self.temperature}°C, Humidity_{self.humidity}%\n")

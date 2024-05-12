"""BMP180 - Air Pressure Sensor

This module contains the BMP180 class which is used to interact with the Bosch BMP180 air pressure sensor.
"""
from machine import I2C
from micropython import const
import struct
import time
import array


BMP180_ADDRESS = const(0x77)
BMP180_CHIP_ID = const(0x55)
BMP180_READ_CHIP_ID = const(0xD0)
BMP180_READ_TEMPERATURE = const(0x2E)
BMP180_READ_PRESSURE_OSS3 = const(0xF4)
BMP180_READ_COEFFICIENTS = const(0xAA)
BMP180_REG_OUT_MSB = const(0xF6)
MESSAGE_EMPTY = const(0x00)


class BMP180():
    def __init__(self, bus: I2C):
        self._command_buffer = bytearray(2)
        self._command_buffer_short = bytearray(1)
        self.bus: I2C = bus
        self.address = BMP180_ADDRESS

        self._raw_coefficients = None
        self._coefficients = array.array('i', (0 for _ in range(11)))
        self._measured_temperature = None
        self._measured_pressure = None

        print(f"Init BMP180(Addr: {self.address}) from I2C Bus {bus}")
        self._read_chip_id()
        self._read_coefficients()
        time.sleep_ms(1000)
    
    def _send_command(self, command, delay: int = 1) -> None:
        if command == BMP180_READ_TEMPERATURE or command == BMP180_READ_PRESSURE_OSS3:
            self._command_buffer[0] = BMP180_READ_PRESSURE_OSS3
            self._command_buffer[1] = command
            # print(f"Send command: {[hex(x) for x in self._command_buffer]}")
        else:
            self._command_buffer_short[0] = command
            # print(f"Send command: {[hex(x) for x in self._command_buffer_short]}")
        try:
            if command == BMP180_READ_TEMPERATURE or command == BMP180_READ_PRESSURE_OSS3:
                self.bus.writeto(self.address, self._command_buffer, True)
            else:
                self.bus.writeto(self.address, self._command_buffer_short, False)
        except OSError as err:
            # [Errno 19] ENODEV: poor contacts (Check the wiring) 
            raise RuntimeError(f"Can't communicate via I2C: {err}")
        time.sleep_ms(delay)

    def _receive_reply(self, length) -> Bytes:
        # No CRC check for BMP180
        try:
            buffer = self.bus.readfrom(self.address, length)
            # print(f"Received: {[hex(x) for x in buffer]}")
            return buffer
        except OSError as err:
            # [Errno 19] ENODEV: poor contacts (Check the wiring) 
            raise RuntimeError(f"Can't communicate via I2C: {err}")
        
    def _read_chip_id(self) -> None:
        self._send_command(BMP180_READ_CHIP_ID)
        buffer = self._receive_reply(1)
        if buffer[0] == BMP180_CHIP_ID:
            print("chip id is 0x55, BMP180 detected")
        else:
            print(f"chip id is: {hex(buffer)}, NOT BMP180!")
    
    def _read_coefficients(self) -> None:
        self._send_command(BMP180_READ_COEFFICIENTS)
        buffer = self._receive_reply(22)
        # collect only power of 2 bytes. 16bit. total 11 coefficients
        # self._coefficients = struct.unpack(">hhhHHHhhhhh", self._message_buffer)
        self._raw_coefficients = buffer
        # print(f"Read coefficients: {self._raw_coefficients=}")

        self._coefficients[0] = struct.unpack_from(">h", self._raw_coefficients)[0] # AC1
        self._coefficients[1] = struct.unpack_from(">h", self._raw_coefficients, 2)[0] # AC2
        self._coefficients[2] = struct.unpack_from(">h", self._raw_coefficients, 4)[0] # AC3
        self._coefficients[3] = struct.unpack_from(">H", self._raw_coefficients, 6)[0] # AC4
        self._coefficients[4] = struct.unpack_from(">H", self._raw_coefficients, 8)[0] # AC5
        self._coefficients[5] = struct.unpack_from(">H", self._raw_coefficients, 10)[0] # AC6
        self._coefficients[6] = struct.unpack_from(">h", self._raw_coefficients, 12)[0] # B1
        self._coefficients[7] = struct.unpack_from(">h", self._raw_coefficients, 14)[0] # B2
        self._coefficients[8] = struct.unpack_from(">h", self._raw_coefficients, 16)[0] # MB
        self._coefficients[9] = struct.unpack_from(">h", self._raw_coefficients, 18)[0] # MC
        self._coefficients[10] = struct.unpack_from(">h", self._raw_coefficients, 20)[0] # MD
    
    def read_temperature(self) -> None:
        self._send_command(BMP180_READ_TEMPERATURE, 5)
        self._send_command(BMP180_REG_OUT_MSB)
        buffer = self._receive_reply(3)
        self._raw_temperature = buffer
        # do not receive data by bytearray
        # print(f"Read temperature: {self._raw_temperature}")
    
    def read_pressure(self) -> None:
        self._send_command(BMP180_READ_PRESSURE_OSS3, 26)
        self._send_command(BMP180_REG_OUT_MSB)
        buffer = self._receive_reply(3)
        self._raw_pressure = buffer
        # print(f"Read pressure: {self._raw_pressure}")
    
    def compute(self, debug: bool = False) -> None:
        # code from sample in this class
        #this is horrible, but it is what the spec sheet says you should do
        
        #int.from_bytes exists, but more limited to struct
        #UT = int.from_bytes(raw_temp, 'big', True)
        UT = struct.unpack_from(">h", self._raw_temperature)[0]
        #Q what do we do with xlsb?
        #UP = struct.unpack_from(">h", raw_press)[0]
        #UP is.. special, time to shift things around
        oss = 3
        UP = self._raw_pressure[0] << 16 | self._raw_pressure[1] << 8 | self._raw_pressure[2]
        UP = UP >> (8 - oss)

        if debug:
            print(f"{UT=}, {UP=}")
            print(f"{self._coefficients=}")

        # compute temperature
        X1 = (UT - self._coefficients[5]) * self._coefficients[4] // 0x8000
        X2 = self._coefficients[9] * 0x0800 // (X1 + self._coefficients[10])
        B5 = X1 + X2
        T = (B5 + 8) // 0x0010
        self._measured_temperature = T / 10

        if debug:
            print(f"{X1=}, {X2=}, {B5=}, {T=}")

        # compute pressure
        B6 = B5 - 4000
        X1 = (self._coefficients[7] * (B6 * B6 // (1 << 12))) // (1 << 11)
        X2 = self._coefficients[1] * B6 // (1 << 11)
        X3 = X1 + X2
        B3 = (((self._coefficients[0] * 4 + X3) << oss) + 2) // 4
        X1 = self._coefficients[2] * B6 // (1 << 13)
        X2 = (self._coefficients[6] * (B6 * B6 // (1 << 12))) // (1 << 16)
        X3 = ((X1 + X2) + 2) // 4
        if debug:
            print(f"{B6=}, {X1=}, {X2=}, {X3=}, {B3=}")

        # unsigned longs here, check later
        B4 = self._coefficients[3] * (X3 + 32768) // (1 << 15)
        B7 = (UP - B3) * (50000 >> 3)
        if B7 < 0x80000000 :
            p = (B7 * 2) // B4
        else:
            p = (B7 // B4) * 2
        X1 = (p // 256) * (p // 256)
        X1 = (X1 * 3038) // (1 << 16)
        X2 = (-7357 * p) // (1 << 16)
        p = p + (X1 + X2 + 3791) // 16

        self._measured_pressure = p / 100

    def read_sensors(self) -> None:
        self.read_temperature()
        self.read_pressure()
        self.compute()

    def print_current_measurement(self) -> None:
        self.read_sensors()
        print(f"[BMP180] Temperature_{self._measured_temperature}°C, Pressure_{self._measured_pressure}hPa")


if __name__ == "__main__":
    from machine import Pin, I2C
    bus = I2C(0, scl=Pin(33, pull=Pin.PULL_UP), sda=Pin(32, pull=Pin.PULL_UP), freq=400000, timeout=1000000)
    scan_res = bus.scan()
    print([hex(x) for x in scan_res])

    bmp180 = BMP180(bus)
    bmp180.print_current_measurement()

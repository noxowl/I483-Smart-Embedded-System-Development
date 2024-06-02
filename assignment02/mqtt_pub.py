"""I483 Assignment 02 1-(a): MQTT Publisher
Author: Suyeong RHIE

The Conditions of Assignment:
- The Data should be published to the MQTT Broker.
- The Topic should be "i483/sensors/[ENTITY]/[SENSOR]/[DATA_TYPE]".
    - ENTITY: Student ID or Team ID
    - SENSOR: BMP180 or SCD41 ...
    - DATA_TYPE: temperature, humidity, co2, air_pressure, illumination, ambient_illumination
- The Data should be published every 15 second.
- The Data should be published in String format.
"""

from machine import Pin, I2C
from micropython import const
from bmp180 import BMP180
from scd41 import SCD41
from umqtt.robust import MQTTClient
# import typing
import ubinascii
import network
import time
import asyncio
import errno
try:
    import umsgpack
    USE_MSGPACK = True
except ImportError:
    import ujson
    USE_MSGPACK = False


# Constants
try:
    """
    The config.json should be stored in the following format:
    {
        "student_id": "YOUR_STUDENT_ID",
        "wifi_ssid": "YOUR_WIFI_SSID",
        "mqtt_broker_url": "YOUR_MQTT_BROKER_URL"
    }
    """
    CONFIG_RAW = open("config.json", "r") # I don't recommend using the JSON for config store but it's built-in. use the yaml instead.
    CONFIG_DATA = ujson.load(CONFIG_RAW)
    CONFIG_RAW.close()
except Exception as e:
    print(f"Failed to load the config.json: {e}")
    raise OSError(errno.ENOENT, "Failed to load the config")
STUDENT_ID = CONFIG_DATA["student_id"]
WIFI_SSID = CONFIG_DATA["wifi_ssid"]
MQTT_BROKER_URL = CONFIG_DATA["mqtt_broker_url"]

INIT_BACKOFF_INTERVAL = const(15)
MAX_BACKOFF_INTERVAL = const(120)
LED_PATTERN_NETWORK_ERROR =  [300, 300, 300, 300, 300]
LED_PATTERN_SENSOR_ERROR = [300, 300, 300]
LED_PATTERN_ERROR = [300]
LED_PATTERN_SUCCESS = [1000]

MQTT_TOPIC_BMP180_TEMPERATURE = f"i483/sensors/{STUDENT_ID}/BMP180/temperature"
MQTT_TOPIC_BMP180_AIR_PRESSURE = f"i483/sensors/{STUDENT_ID}/BMP180/air_pressure"
MQTT_TOPIC_SCD41_HUMIDITY = f"i483/sensors/{STUDENT_ID}/SCD41/humidity"
MQTT_TOPIC_SCD41_CO2 = f"i483/sensors/{STUDENT_ID}/SCD41/co2"
MQTT_TOPIC_SCD41_TEMPERATURE = f"i483/sensors/{STUDENT_ID}/SCD41/temperature"
MQTT_TOPIC_JSON = f"i483/sensors/{STUDENT_ID}/json"
MQTT_TOPIC_MSGPACK = f"i483/sensors/{STUDENT_ID}/msgpack"


def connect_wifi(wifi: network.WLAN, wifi_ssid: str, backoff_interval: int = INIT_BACKOFF_INTERVAL): #-> typing.Union[None, int]:
    # connect wifi with exponential backoff
    current_counts = 0
    while not wifi.isconnected() & (backoff_interval <= MAX_BACKOFF_INTERVAL):
        try:
            if wifi.status() == network.STAT_IDLE:
                print("Connecting to WiFi...")
                wifi.active(True)
                print("The MAC Address: ", ubinascii.hexlify(wifi.config("mac"), ':').decode().upper())
                wifi.connect(wifi_ssid)
            elif wifi.status() == network.STAT_CONNECTING:
                print(f"Connecting... {current_counts}/{backoff_interval}")
                time.sleep(1)
                current_counts += 1
                if current_counts >= backoff_interval:
                    backoff_interval = backoff_interval * 2
                    current_counts = 0
            elif wifi.status() == network.STAT_WRONG_PASSWORD:
                print("Failed to connect to WiFi: Wrong Password")
                return errno.ECONNREFUSED
            else:
                print(f"status: {wifi.status()}")
                break
        except Exception as e:
            print(f"Failed to connect to WiFi: {e}")
            return errno.ECONNREFUSED
    if not wifi.isconnected():
        print("Failed to connect to WiFi")
        return errno.ECONNREFUSED
    print("Connected to WiFi")
    return None


def blink_led(led: Pin, pattern: list[float]) -> None:
    # blink the LED
    for duration in pattern:
        led.on()
        time.sleep_ms(duration)
        led.off()
        time.sleep_ms(duration)
    

def on_error(error: int = errno.EPERM) -> None:
    # blink the LED to indicate an error
    # The micropython does not support the match statement (PEP 634)
    # https://github.com/micropython/micropython/issues/7847
    pin = Pin(2, Pin.OUT)
    print(f"Error: {error} {errno.errorcode[error]}")
    while True:
        if error == errno.ENODEV:
            blink_led(pin, LED_PATTERN_SENSOR_ERROR)
        elif error == errno.ECONNREFUSED:
            blink_led(pin, LED_PATTERN_NETWORK_ERROR)
        else:
            blink_led(pin, LED_PATTERN_ERROR)
        time.sleep(1)


def on_success() -> None:
    # blink the LED to indicate a success
    pin = Pin(2, Pin.OUT)
    blink_led(pin, LED_PATTERN_SUCCESS)


def _init_sensors(bus: I2C): #-> typing.Union[tuple[BMP180, SCD41], int]:
    # Initialize the sensors
    try:
        bmp180 = BMP180(bus)
        scd41 = SCD41(bus)
        return bmp180, scd41
    except Exception as e:
        print(f"Failed to initialize the sensors: {e}")
        return errno.ENODEV


async def publish_data(client, topic, data) -> None:
    # publish the data to the MQTT Broker
    try:
        if not isinstance(data, bytes):
            data = f"{data}".encode()
        client.publish(topic, data)
    except Exception as e:
        print(f"Failed to publish the data: {e}")
        on_error() # Inconsistent, but due to time constraints.


async def publish_sensor_data(client, bmp180: BMP180, scd41: SCD41) -> None:
    # publish the sensor data to the MQTT Broker
    while True:
        try:
            data = {
                "bmp180": {
                    "temperature": bmp180.temperature,
                    "air_pressure": bmp180.air_pressure
                },
                "scd41": {
                    "humidity": scd41.humidity,
                    "co2": scd41.co2,
                    "temperature": scd41.temperature
                }
            }
            print(data)
            scd41.periodic_measurements(False)
            if USE_MSGPACK:
                await publish_data(client, MQTT_TOPIC_MSGPACK, umsgpack.packb(data))
            else:
                await publish_data(client, MQTT_TOPIC_JSON, ujson.dumps(data))
            await publish_data(client, MQTT_TOPIC_BMP180_TEMPERATURE, data["bmp180"]["temperature"])
            await publish_data(client, MQTT_TOPIC_BMP180_AIR_PRESSURE, data["bmp180"]["air_pressure"])
            if data["scd41"]["humidity"]:
                await publish_data(client, MQTT_TOPIC_SCD41_HUMIDITY, data["scd41"]["humidity"])
                await publish_data(client, MQTT_TOPIC_SCD41_CO2, data["scd41"]["co2"])
                await publish_data(client, MQTT_TOPIC_SCD41_TEMPERATURE, data["scd41"]["temperature"])
            on_success() # sleep 2 seconds
            scd41.periodic_measurements(True)
            await asyncio.sleep(13) # + on_success = 15 seconds
        except Exception as e:
            print(f"Failed to publish the sensor data: {e}")
            on_error() # Inconsistent, but due to time constraints.


async def publish_serialized_data(client, bmp180: BMP180, scd41: SCD41) -> None:
    # publish the sensor data to the MQTT Broker in JSON/Msgpack format
    while True:
        try:
            data = {
                "bmp180": {
                    "temperature": bmp180.get_temperature(),
                    "air_pressure": bmp180.get_air_pressure()
                },
                "scd41": {
                    "humidity": scd41.get_humidity(),
                    "co2": scd41.get_co2(),
                    "temperature": scd41.get_temperature()
                }
            }
            if USE_MSGPACK:
                publish_data(client, MQTT_TOPIC_MSGPACK, umsgpack.packb(data))
            else:
                publish_data(client, MQTT_TOPIC_JSON, ujson.dumps(data))
            await asyncio.sleep(15)
        except Exception as e:
            print(f"Failed to publish the Serialized data: {e}")
            on_error() # Inconsistent, but due to time constraints.


async def main(client: MQTTClient, bmp180: BMP180, scd41: SCD41):
    # Publish the sensor data
    await publish_sensor_data(client, bmp180, scd41)
    # Publish the sensor data in JSON/Msgpack format
    # await publish_serialized_data(client, bmp180, scd41)
    # merged to the publish_sensor_data


if __name__ == "__main__":
    print("I483 Assignment 02 1-(a): MQTT Publisher")
    bus = I2C(0, scl=Pin(33, pull=Pin.PULL_UP), sda=Pin(32, pull=Pin.PULL_UP), freq=400000, timeout=1000000)
    wifi = network.WLAN(network.STA_IF)
    # Connect to the WiFi
    error = connect_wifi(wifi, WIFI_SSID)
    if error:
        on_error(error)
    # Initialize the sensors
    result = _init_sensors(bus)
    if isinstance(result, int):
        on_error(result)
    bmp180, scd41 = result

    # Connect to the MQTT Broker
    client = MQTTClient(client_id=STUDENT_ID, server=MQTT_BROKER_URL)
    client.connect()
    # Run the main function
    asyncio.run(main(client, bmp180, scd41))



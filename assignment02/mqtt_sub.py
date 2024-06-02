"""I483 Assignment 02 2-(b): MQTT Subscriber
Author: Suyeong Rhie

The Conditions of Assignment:
- The Data should be subscribed from the MQTT Broker.
"""
import argparse
from paho.mqtt.client import Client as MQTTClient


def subscribe_callback(client, userdata, message):
    print(f"Received message '{message.payload.decode()}' on topic '{message.topic}'")


def on_connect(client, userdata, flags, rc):
    print(f"Connected with result code {rc}")


def on_disconnect(client, userdata, rc):
    print(f"Disconnected with result code {rc}")


if __name__ == "__main__":
    print("I483 Assignment 02 2-(b): MQTT Subscriber")
    parser = argparse.ArgumentParser(description="MQTT Subscriber")
    parser.add_argument("--broker", type=str, required=True, help="MQTT Broker Address")
    parser.add_argument("--client_id", type=str, required=True, help="MQTT Client ID")
    parser.add_argument("--topics", type=str, nargs='*', required=True, help="MQTT Topic")
    args = parser.parse_args()

    client = MQTTClient(client_id=args.client_id)
    client.on_connect = on_connect
    client.on_disconnect = on_disconnect
    client.on_message = subscribe_callback
    client.connect(args.broker)
    for topic in args.topics:
        client.subscribe(topic)

    client.loop_forever()
        
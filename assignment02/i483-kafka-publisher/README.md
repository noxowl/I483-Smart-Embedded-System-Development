# I483-Kafka-Project

## Project Description

This project is a simple implementation of a Kafka producer and consumer. The consumer reads messages from a Kafka topic and the producer sends messages to the new topic. The producer and consumer are implemented in Rust using the `rdkafka` library.

## How to run
* This project requires a librdkafka library for the Rust bindings to work. You can install it by running `sudo port install librdkafka` on macOS.
* To run the listener for consuming only, run `cargo run --bin i483-kafka-publisher listen --host <HOST> --topics <TOPICS_TO_CONSUME>`. 
* To run the listener for consuming and producing, run `cargo run --bin i483-kafka-publisher process --host <HOST> --topics <TOPICS_TO_CONSUME> --processes <PROCESS:ARGUMENT>`.
* The `--processes` flag must be paired with a `--topics` flag. The `--processes` flag takes a string of the form `<PROCESS:ARGUMENT>`. The `ARGUMENT` must be a positive integer. 
* Multiple arguments are separated by a space. For example, `--topics topic1 topic2 topic3` or `--processes "rolling-average:10 threshold:20 threshold:30`.
* Description of the processes:
  * `rolling-average`: Calculates the rolling average of the last `<DURATION>` minutes of messages.
  * `threshold`: Checks if the message is greater than the threshold value. The threshold value is specified in the argument.

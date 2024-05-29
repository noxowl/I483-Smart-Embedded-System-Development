mod cli;

use std::env;
use paho_mqtt as mqtt;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mqtt_host = &args[1];

}

mod cli;
mod mqtt;

use std::env;
use futures::executor::block_on;
use rand::distributions::{Alphanumeric, DistString};


#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let command = cli::parse_args(args);

    match command {
        cli::Command::Help => {
            cli::print_usage();
        }
        cli::Command::Listen(args) => {
            let client_id = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            let (client, mut event_loop) = block_on(mqtt::connect(&client_id, &args.host)).unwrap();
            block_on(mqtt::subscribe(&client, &args.topics)).unwrap(); //&args.topics.iter().map(|s| s.as_str()).collect())
            block_on(mqtt::listen(&mut event_loop)).unwrap();
        }
    }
}

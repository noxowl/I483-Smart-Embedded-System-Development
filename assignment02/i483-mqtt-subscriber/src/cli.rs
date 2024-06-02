

pub enum Command {
    Help,
    Listen(Args),
}

pub fn print_usage() {
    println!("Usage: mqtt-cli listen <host> <topic1> <topic2> ...");
}

pub struct Args {
    pub host: String,
    pub topics: Vec<String>,
}

pub fn parse_args(args: Vec<String>) -> Command {
    if args.len() < 2 {
        return Command::Help;
    }

    let command = &args[1];

    match command.as_str() {
        "listen" => {
            if args.len() < 3 {
                Command::Help
            } else {
                let host = args[2].clone();
                let topics = args[3..].to_vec();
                Command::Listen(Args { host, topics })
            }
        }
        _ => Command::Help,
    }
}

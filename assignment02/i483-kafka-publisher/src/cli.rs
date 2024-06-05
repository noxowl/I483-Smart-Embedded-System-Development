use anyhow::anyhow;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help,
    Listen(Args),
    Process(Args),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessType {
    RollingAverage(u64),
    Threshold(u64), // When the average is above this threshold, send an alert. and when it's below, send a recovery alert.
}

pub fn print_usage() {
    println!("Usage 1: kafka-publisher listen --host <host> --topics <topic1> <topic2> ...");
    println!("Usage 2: kafka-publisher process --host <host> --topics <topic1> --processes <process1> ...");
}

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub host: String,
    pub topics: Vec<String>,
    pub processes: Vec<ProcessType>,
    pub debug: bool,
    pub dry_run: bool,
}

pub fn parse_args(args: Vec<String>) -> Command {
    if args.len() < 2 {
        return Command::Help;
    }
    let mut host = String::new();
    let mut topics = Vec::new();
    let mut processes = Vec::new();
    let mut debug = false;
    let mut dry_run = false;

    let mut cursor = 0;

    while cursor < args.len(){
        let arg = &args[cursor];
        match arg.as_str() {
            "--host" => {
                host = args.get(cursor + 1).ok_or(anyhow!("no argument found for option")).unwrap().clone();
                if host.contains("--") {
                    return Command::Help;
                }
            },
            "--topics" => {
                let mut seeking = true;
                while seeking {
                    cursor += 1;
                    if cursor >= args.len() {
                        seeking = false;
                    } else {
                        let topic = args.get(cursor).ok_or(anyhow!("no argument found for option")).unwrap().clone();
                        if topic.contains("--") {
                            cursor -= 1;
                            seeking = false;
                        } else {
                            topics.push(topic);
                        }
                    }
                }
            },
            "--processes" => {
                let mut seeking = true;
                while seeking {
                    cursor += 1;
                    if cursor >= args.len() {
                        seeking = false;
                    } else {
                        let process = args.get(cursor).ok_or(anyhow!("no argument found for option")).unwrap().clone();
                        if process.contains("--") {
                            cursor -= 1;
                            seeking = false;
                        } else {
                            processes.push(parse_process(&process));
                        }
                    }
                }
            },
            "--debug" => {
                debug = true;
            },
            "--dry-run" => {
                dry_run = true;
            },
            _ => {},
        }
        cursor += 1;
    }

    println!("Initializing with host: {}, topics: {:?}, processes: {:?}, debug: {}, dry_run: {}", host, topics, processes, debug, dry_run);
    match args[1].as_str() {
        "listen" => Command::Listen(Args { host, topics, processes, debug, dry_run }),
        "process" => Command::Process(Args { host, topics, processes, debug, dry_run }),
        _ => Command::Help,
    }
}


fn parse_process(process: &str) -> ProcessType {
    let mut parts = process.split(":");
    let process_type = parts.next().unwrap();
    let process_value = parts.next().unwrap().parse().unwrap();
    match process_type.to_ascii_lowercase().as_str() {
        "rolling-average" => ProcessType::RollingAverage(process_value),
        "threshold" => ProcessType::Threshold(process_value),
        _ => ProcessType::RollingAverage(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_process() {
        assert_eq!(parse_process("rolling-average:10"), ProcessType::RollingAverage(10));
        assert_eq!(parse_process("threshold:10"), ProcessType::Threshold(10));
        assert_eq!(parse_process("invalid:10"), ProcessType::RollingAverage(0));
    }

    #[test]
    fn test_parse_args() {
        let args = vec![
            "kafka-publisher".to_string(),
            "listen".to_string(),
            "--host".to_string(),
            "localhost:9092".to_string(),
            "--topics".to_string(),
            "topic1".to_string(),
            "topic2".to_string(),
        ];
        match parse_args(args) {
            Command::Listen(Args { host, topics, processes, debug: false, dry_run: false }) => {
                assert_eq!(host, "localhost:9092");
                assert_eq!(topics, vec!["topic1".to_string(), "topic2".to_string()]);
                assert_eq!(processes, vec![]);
            },
            _ => panic!("unexpected command"),
        }
    }

    #[test]
    fn test_parse_args_with_process() {
        let args = vec![
            "kafka-publisher".to_string(),
            "process".to_string(),
            "--host".to_string(),
            "localhost:9092".to_string(),
            "--topics".to_string(),
            "topic1".to_string(),
            "topic2".to_string(),
            "--processes".to_string(),
            "rolling-average:10".to_string(),
            "threshold:20".to_string(),
        ];
        match parse_args(args) {
            Command::Process(Args { host, topics, processes, debug: false, dry_run: false }) => {
                assert_eq!(host, "localhost:9092");
                assert_eq!(topics, vec!["topic1".to_string(), "topic2".to_string()]);
                assert_eq!(processes, vec![ProcessType::RollingAverage(10), ProcessType::Threshold(20)]);
            },
            _ => panic!("unexpected command"),
        }
    }

    #[test]
    fn test_parse_args_with_mixed_args_order() {
        let args = vec![
            "kafka-publisher".to_string(),
            "process".to_string(),
            "--topics".to_string(),
            "topic1".to_string(),
            "topic2".to_string(),
            "--host".to_string(),
            "localhost:9092".to_string(),
            "--processes".to_string(),
            "rolling-average:10".to_string(),
            "threshold:20".to_string(),
        ];
        match parse_args(args) {
            Command::Process(Args { host, topics, processes, debug: false, dry_run: false }) => {
                assert_eq!(host, "localhost:9092");
                assert_eq!(topics, vec!["topic1".to_string(), "topic2".to_string()]);
                assert_eq!(processes, vec![ProcessType::RollingAverage(10), ProcessType::Threshold(20)]);
            },
            _ => panic!("unexpected command"),
        }
    }

    #[test]
    fn test_parse_args_with_invalid_args() {
        let args = vec![
            "kafka-publisher".to_string(),
            "process".to_string(),
            "--topics".to_string(),
            "topic1".to_string(),
            "topic2".to_string(),
            "--host".to_string(),
            "--processes".to_string(),
            "rolling-average:10".to_string(),
            "threshold:20".to_string(),
        ];
        match parse_args(args) {
            Command::Help => {},
            _ => panic!("unexpected command"),
        }
    }
}

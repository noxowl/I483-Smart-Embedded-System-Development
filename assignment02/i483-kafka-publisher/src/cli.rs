use anyhow::anyhow;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help,
    Listen(Args),
    Process(Args),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Process {
    RollingAverage(u32),
    Threshold(u32), // When the average is above this threshold, send an alert. and when it's below, send a recovery alert.
}

pub fn print_usage() {
    println!("Usage 1: kafka-publisher listen --host <host> --topics <topic1> <topic2> ...");
    println!("Usage 2: kafka-publisher process --host <host> --topics <topic1> --processes <process1> ...");
}

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub host: String,
    pub topics: Vec<String>,
    pub processes: Vec<Process>,
}

pub fn parse_args(args: Vec<String>) -> Command {
    if args.len() < 2 {
        return Command::Help;
    }
    let mut host = String::new();
    let mut topics = Vec::new();
    let mut processes = Vec::new();

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
                for (j, topic) in args[cursor..].iter().enumerate() {
                    cursor = j;
                    if topic.contains("--") {
                        break;
                    }
                    topics.push(topic.clone());
                }
            },
            "--processes" => {
                for (j, process) in args[cursor..].iter().enumerate() {
                    cursor = j;
                    if process.contains("--") {
                        break;
                    }
                    processes.push(parse_process(process));
                }
            },
            _ => {},
        }
        cursor += 1;
    }

    match args[1].as_str() {
        "listen" => Command::Listen(Args { host, topics, processes }),
        "process" => Command::Process(Args { host, topics, processes }),
        _ => Command::Help,
    }
}


fn parse_process(process: &str) -> Process {
    let mut parts = process.split(":");
    let process_type = parts.next().unwrap();
    let process_value = parts.next().unwrap().parse().unwrap();
    match process_type {
        "rolling-average" => Process::RollingAverage(process_value),
        "threshold" => Process::Threshold(process_value),
        _ => Process::RollingAverage(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_process() {
        assert_eq!(parse_process("rolling-average:10"), Process::RollingAverage(10));
        assert_eq!(parse_process("threshold:10"), Process::Threshold(10));
        assert_eq!(parse_process("invalid:10"), Process::RollingAverage(0));
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
            Command::Listen(Args { host, topics, processes }) => {
                assert_eq!(host, "localhost:9092");
                assert_eq!(topics, vec!["topic1".to_string(), "topic2".to_string()]);
                assert_eq!(processes, vec![]);
            },
            _ => panic!("unexpected command"),
        }
    }
}

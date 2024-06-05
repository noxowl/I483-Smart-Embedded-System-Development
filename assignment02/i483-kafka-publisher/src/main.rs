mod cli;
mod kafka;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = cli::parse_args(args);

    match command {
        cli::Command::Help => {
            cli::print_usage();
        }
        cli::Command::Process(args) => {
            // kafka::process(&args.host, &args.topics, &args.processes).await;
        }
        cli::Command::Listen(args) => {
            kafka::listen(&args.host, &args.topics).await;
        }
    }
}

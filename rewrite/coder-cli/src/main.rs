use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "")] 
    message: String,
}

#[tokio::main]
async fn main() {
    let _ = env_logger::try_init();
    let args = Args::parse();
    if args.message.is_empty() {
        println!("coder-cli: hello from scaffold");
    } else {
        println!("Message: {}", args.message);
    }
}

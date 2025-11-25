use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "hello", about = "Usage: hello [OPTIONS] [NAME]", author, version)]
struct Cli{
    name: Option<String>,

    #[arg(long)]
    upper: bool,

    #[arg(long, default_value_t = 1)]
    repeat: u32,
}
fn main() {
    let args = Cli::parse();

    let name = args.name.unwrap_or_else(|| "world".to_string());

    let mut greeting = format!("Hello, {}!", name);

    if args.upper {
        greeting = greeting.to_uppercase();
    }

    for _ in 0..args.repeat {
        println!("{}", greeting);
    }
}


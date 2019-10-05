use std::env;
use std::process;

use crtime::Config;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem with parsing arguments: {}", err);
        process::exit(1);
    });

    if let Err(err) = crtime::run(config) {
        eprintln!("Application error: {}", err);
        process::exit(1);
    }
}

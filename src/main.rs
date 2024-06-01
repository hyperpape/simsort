use simsort::*;

use clap::Parser;

fn main() {
    env_logger::init();
    let args = Args::parse();
    match run(args) {
        Ok(_) => {
            utils::perf_trace("Simsort", "Process", "E", utils::get_micros());
            std::process::exit(exitcode::OK);
        }
        Err(code) => {
            utils::perf_trace("Simsort", "Process", "E", utils::get_micros());
            std::process::exit(code);
        }
    }
}
use std::process::ExitCode;

fn main() -> ExitCode {
    match pwg::cli::args::parse(std::env::args_os().skip(1)).and_then(pwg::run) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

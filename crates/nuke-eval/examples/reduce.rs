use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: reduce <file.nuke>");
        return ExitCode::FAILURE;
    };
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let value = match nuke_eval::eval(&source) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{}:{}: {error}", path.display(), error.location(&source));
            return ExitCode::FAILURE;
        }
    };
    match nuke_transpile::json::to_string(&value) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
}

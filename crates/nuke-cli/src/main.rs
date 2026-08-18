use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use nuke_transpile::Target;

const EXTENSION: &str = "nuke";

#[derive(Parser)]
#[command(
    name = "nuke",
    version,
    about = "Write a Nuke document out as a target"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Render {
        #[arg(short, long, value_name = "TARGET", value_parser = Target::from_str)]
        format: Option<Target>,
        file: PathBuf,
    },
    Deps {
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Render { format, file } => render(format, &file),
        Command::Deps { file } => deps(&file),
    }
    .unwrap_or_else(|report| {
        eprintln!("{report}");
        ExitCode::FAILURE
    })
}

fn render(format: Option<Target>, file: &Path) -> Result<ExitCode, String> {
    let target = match format {
        Some(target) => target,
        None => named_by(file)?,
    };
    let source = read(file)?;
    let value = reduce(&source, file)?;
    let text = target
        .render(&value)
        .map_err(|refusal| format!("{}: {refusal}", file.display()))?;
    println!("{text}");
    Ok(ExitCode::SUCCESS)
}

fn deps(file: &Path) -> Result<ExitCode, String> {
    let source = read(file)?;
    let reduction = nuke_eval::eval_at_with_files(&source, file)
        .map_err(|error| located(file, &source, &error))?;
    for read in reduction.files {
        println!("{}", read.display());
    }
    Ok(ExitCode::SUCCESS)
}

fn read(file: &Path) -> Result<String, String> {
    fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))
}

fn reduce(source: &str, file: &Path) -> Result<nuke_eval::Value, String> {
    nuke_eval::eval_at(source, file).map_err(|error| located(file, source, &error))
}

fn located(file: &Path, source: &str, error: &nuke_eval::Error) -> String {
    format!("{}:{}: {error}", file.display(), error.location(source))
}

fn named_by(file: &Path) -> Result<Target, String> {
    let stem = file
        .extension()
        .filter(|extension| *extension == EXTENSION)
        .and_then(|_| file.file_stem())
        .map(Path::new)
        .ok_or_else(|| unnamed(file))?;
    stem.extension()
        .and_then(|extension| extension.to_str())
        .and_then(Target::from_extension)
        .ok_or_else(|| unnamed(file))
}

fn unnamed(file: &Path) -> String {
    let targets: Vec<&str> = Target::ALL.iter().map(|target| target.name()).collect();
    format!(
        "{} does not name a target, so `--format` is needed: one of {}",
        file.display(),
        targets.join(", ")
    )
}

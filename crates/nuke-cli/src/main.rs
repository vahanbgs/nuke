use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use nuke_transpile::Target;

const EXTENSION: &str = "nuke";

const STDIN: &str = "-";

#[derive(Parser)]
#[command(
    name = "nuke",
    version,
    about = "Render a Nuke document, list what it reads, format it, check its style and serve an editor"
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
    Fmt {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    Lint {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    Lsp,
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Render { format, file } => render(format, &file),
        Command::Deps { file } => deps(&file),
        Command::Fmt { file } => fmt(&file),
        Command::Lint { file } => lint(&file),
        Command::Lsp => lsp(),
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

fn fmt(file: &Path) -> Result<ExitCode, String> {
    let source = source(file)?;
    let formatted = nuke_syntax::printer::format(&source)
        .map_err(|error| format!("{}:{}: {error}", name(file), error.location(&source)))?;
    print!("{formatted}");
    Ok(ExitCode::SUCCESS)
}

fn lint(file: &Path) -> Result<ExitCode, String> {
    let source = source(file)?;
    let found = nuke_lint::lint(&source)
        .map_err(|error| format!("{}:{}: {error}", name(file), error.location(&source)))?;
    for diagnostic in &found {
        println!(
            "{}:{}: {}: {diagnostic}",
            name(file),
            diagnostic.location(&source),
            diagnostic.rule()
        );
    }
    Ok(if found.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn lsp() -> Result<ExitCode, String> {
    nuke_lsp::serve()
        .map(|()| ExitCode::SUCCESS)
        .map_err(|error| format!("lsp: {error}"))
}

fn source(file: &Path) -> Result<String, String> {
    if file == Path::new(STDIN) {
        let mut source = String::new();
        return std::io::stdin()
            .read_to_string(&mut source)
            .map(|_| source)
            .map_err(|error| format!("{}: {error}", name(file)));
    }
    read(file)
}

fn name(file: &Path) -> String {
    if file == Path::new(STDIN) {
        "<stdin>".to_owned()
    } else {
        file.display().to_string()
    }
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

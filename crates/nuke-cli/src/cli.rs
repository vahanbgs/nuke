use std::path::PathBuf;
use std::str::FromStr;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Parser, Subcommand, ValueHint};
use nuke_transpile::Target;

#[derive(Parser)]
#[command(
    name = "nuke",
    version,
    about = "Render a Nuke document, list what it reads, format it, check its style and serve an editor"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Reduce a document and write its target's text")]
    Render {
        #[arg(
            short,
            long,
            value_name = "TARGET",
            value_parser = target(),
            help = "The target to write, when the file's name does not say"
        )]
        format: Option<Target>,
        #[arg(
            value_name = "FILE",
            value_hint = ValueHint::FilePath,
            help = "The document to read"
        )]
        file: PathBuf,
    },
    #[command(about = "List the entry file and every file its reduction read")]
    Deps {
        #[arg(
            value_name = "FILE",
            value_hint = ValueHint::FilePath,
            help = "The document to read"
        )]
        file: PathBuf,
    },
    #[command(about = "Print a document in canonical style, leaving the file alone")]
    Fmt {
        #[arg(
            value_name = "FILE",
            value_hint = ValueHint::FilePath,
            help = "The document to read, or `-` for stdin"
        )]
        file: PathBuf,
    },
    #[command(about = "Report the style the grammar admits and the convention does not")]
    Lint {
        #[arg(
            value_name = "FILE",
            value_hint = ValueHint::FilePath,
            help = "The document to read, or `-` for stdin"
        )]
        file: PathBuf,
    },
    #[command(about = "Serve an editor over stdio")]
    Lsp,
}

fn target() -> impl TypedValueParser<Value = Target> {
    PossibleValuesParser::new(Target::ALL.map(Target::name)).try_map(|name| Target::from_str(&name))
}

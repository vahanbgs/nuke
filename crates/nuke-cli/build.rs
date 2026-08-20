use std::env;
use std::io::Error;

use clap::{CommandFactory, ValueEnum};
use clap_complete::Shell;
use clap_complete_nushell::Nushell;

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let out = env::var_os("OUT_DIR").expect("cargo should set OUT_DIR for a build script");
    let mut command = Cli::command();
    for shell in Shell::value_variants() {
        clap_complete::generate_to(*shell, &mut command, "nuke", &out)?;
    }
    clap_complete::generate_to(Nushell, &mut command, "nuke", &out)?;
    clap_mangen::generate_to(Cli::command(), &out)?;
    println!("cargo::rerun-if-changed=src/cli.rs");
    Ok(())
}

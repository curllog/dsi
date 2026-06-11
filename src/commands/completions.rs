// src/commands/completions.rs
//
// `dsi completions <shell>` — Generate shell completion scripts.

use std::io;

use anyhow::Result as AnyResult;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;

#[derive(Parser)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

pub fn run<C: CommandFactory>(args: CompletionsArgs) -> AnyResult<()> {
    let mut cmd = C::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(args.shell, &mut cmd, name, &mut io::stdout());
    Ok(())
}

use clap::Parser;
use commands::info;
mod commands;
mod platform;
/// A fast CLI tool for installing and managing .NET SDK versions.
#[derive(Parser)]
#[command(name="dsi", version, about)]
struct Cli {
 #[command(subcommand)]
 command:Commands,
}
#[derive(clap::Subcommand)]
enum Commands{

    /// Show environment and platform details
    Info,
}
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Info =>{
            info::run();
        }
    }
}
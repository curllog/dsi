use clap::Parser;
mod api;
mod commands;
mod platform;
/// A fast CLI tool for installing and managing .NET SDK versions.
#[derive(Parser)]
#[command(name = "dsi", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(clap::Subcommand)]
enum Commands {
    /// Show environment and platform details
    Info,
    ///List all installed SDKs

    /// List available SDK versions from Microsoft's releases API
    Ls,
    LsRemote(commands::ls_remote::LsRemoteArgs),
}
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Info => commands::info::run(),

        Commands::LsRemote(args) => commands::ls_remote::run(args).await,
        Commands::Ls => commands::ls::run().await,
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        for cause in e.chain().skip(1) {
            eprintln!(" caused by: {}", cause);
        }
        std::process::exit(1);
    }
}

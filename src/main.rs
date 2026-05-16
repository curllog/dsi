use clap::Parser;

mod api;
mod commands;
mod paths;
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
    /// List available SDK versions from Microsoft's releases API
    Ls(commands::ls::LsArgs),
    ///List all installed SDKs
    LsRemote(commands::ls_remote::LsRemoteArgs),
    // Install Sdk
    Install(commands::install::InstallArgs),
}
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Info => commands::info::run(),
        Commands::LsRemote(args) => commands::ls_remote::run(args).await,
        Commands::Ls(args) => commands::ls::run(args).await,
        Commands::Install(args) => commands::install::run(args).await,
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        for cause in e.chain().skip(1) {
            eprintln!(" caused by: {}", cause);
        }
        std::process::exit(1);
    }
}

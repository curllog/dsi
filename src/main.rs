use clap::Parser;
mod api;
mod commands;
mod paths;
mod platform;
mod prompt;
mod version;
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
    /// Remove a specific installed SDK version
    Uninstall(commands::uninstall::UninstallArgs),
    /// Update installed SDKs to the latest patch in their channel
    Update(commands::update::UpdateArgs),
    /// Remove outdated patches, keeping the latest per feature band
    Prune(commands::prune::PruneArgs),
    /// Update dsi itself to the latest release (not yet implemented)
    Selfupdate(commands::selfupdate::SelfUpdateArgs),
    /// Remove the dsi binary, leaving installed SDKs intact
    Selfuninstall(commands::selfuninstall::SelfUninstallArgs),
}
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Info => commands::info::run(),
        Commands::LsRemote(args) => commands::ls_remote::run(args).await,
        Commands::Ls(args) => commands::ls::run(args).await,
        Commands::Install(args) => commands::install::run(args).await,
        Commands::Uninstall(args) => commands::uninstall::run(args).await,
        Commands::Update(args) => commands::update::run(args).await,
        Commands::Prune(args) => commands::prune::run(args).await,
        Commands::Selfupdate(args) => commands::selfupdate::run(args).await,
        Commands::Selfuninstall(args) => commands::selfuninstall::run(args).await,
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        for cause in e.chain().skip(1) {
            eprintln!(" caused by: {}", cause);
        }
        std::process::exit(1);
    }
}

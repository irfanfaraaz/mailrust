// src/cli.rs
use clap::{Parser, Subcommand, ValueEnum};

use crate::{config::Config, db, telemetry};

#[derive(Parser)]
#[command(name = "mailrust", version, about = "Self-hosted email engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Role {
    All,
    Api,
    Worker,
}

#[derive(Subcommand)]
enum Command {
    /// Apply database migrations
    Migrate,
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::from_env()?;
    telemetry::init(config.log_json);
    let pool = db::connect(&config.database_url).await?;

    match cli.command {
        Command::Migrate => {
            db::migrate(&pool).await?;
            println!("migrations applied");
        }
    }
    Ok(())
}

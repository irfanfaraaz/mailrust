// src/cli.rs
use clap::{Parser, Subcommand, ValueEnum};

use uuid::Uuid;

use crate::{
    accounts,
    config::Config,
    db,
    keys::{self, KeyAccess},
    sender, telemetry,
};

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
    /// Manage accounts
    #[command(subcommand)]
    Account(AccountCmd),
    /// Manage API keys
    #[command(subcommand)]
    Key(KeyCmd),
}

#[derive(Subcommand)]
enum AccountCmd {
    /// Create an account and set it up at the sender
    Create {
        #[arg(long)]
        name: String,
    },
    List,
    Pause {
        id: Uuid,
        #[arg(long, default_value = "paused by operator")]
        reason: String,
    },
    Resume {
        id: Uuid,
    },
    /// Create or repair the account's sender setup (safe to run again)
    SyncSes {
        id: Option<Uuid>,
        #[arg(long)]
        all: bool,
    },
}

#[derive(Subcommand)]
enum KeyCmd {
    /// Create a key. It is printed once and never shown again.
    Create {
        #[arg(long)]
        account: Uuid,
        #[arg(long)]
        name: String,
        #[arg(long, value_enum, default_value_t = KeyAccess::Full)]
        access: KeyAccess,
    },
    Revoke {
        id: Uuid,
    },
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
        Command::Account(cmd) => account_cmd(&pool, &config, cmd).await?,
        Command::Key(cmd) => key_cmd(&pool, cmd).await?,
    }
    Ok(())
}

async fn account_cmd(pool: &sqlx::PgPool, config: &Config, cmd: AccountCmd) -> anyhow::Result<()> {
    match cmd {
        AccountCmd::Create { name } => {
            let name = name.trim();
            anyhow::ensure!(!name.is_empty() && name.len() <= 100, "name must be 1-100 characters");
            let sender = sender::from_config(config).await?;
            let account = accounts::create(pool, name).await?;
            println!("account {} created ({})", account.id, account.name);
            match accounts::provision(pool, sender.as_ref(), &account).await {
                Ok(()) => println!("sender setup done"),
                Err(e) => eprintln!(
                    "warning: sender setup failed: {e:#}\nrun `mailrust account sync-ses {}` later",
                    account.id
                ),
            }
        }
        AccountCmd::List => {
            for a in accounts::list(pool).await? {
                println!(
                    "{}  {:<8} {}  synced={}",
                    a.id,
                    format!("{:?}", a.status).to_lowercase(),
                    a.name,
                    a.ses_synced_at.is_some()
                );
            }
        }
        AccountCmd::Pause { id, reason } => {
            let changed = accounts::pause(pool, id, &reason).await?;
            println!("{}", if changed { "paused" } else { "not changed (missing or already paused)" });
        }
        AccountCmd::Resume { id } => {
            let changed = accounts::resume(pool, id).await?;
            println!("{}", if changed { "resumed" } else { "not changed (missing or not paused)" });
        }
        AccountCmd::SyncSes { id, all } => {
            let sender = sender::from_config(config).await?;
            let targets = match (id, all) {
                (Some(id), false) => vec![accounts::get(pool, id).await?.ok_or_else(|| anyhow::anyhow!("no account {id}"))?],
                (None, true) => accounts::list(pool).await?,
                _ => anyhow::bail!("pass one account id, or --all"),
            };
            for a in targets {
                match accounts::provision(pool, sender.as_ref(), &a).await {
                    Ok(()) => println!("{} synced", a.id),
                    Err(e) => eprintln!("{} failed: {e:#}", a.id),
                }
            }
        }
    }
    Ok(())
}

async fn key_cmd(pool: &sqlx::PgPool, cmd: KeyCmd) -> anyhow::Result<()> {
    match cmd {
        KeyCmd::Create { account, name, access } => {
            anyhow::ensure!(accounts::get(pool, account).await?.is_some(), "no account {account}");
            let key = keys::create(pool, account, &name, access).await?;
            println!("key id: {}\nkey:    {}\n(save it now, it is not shown again)", key.id, key.token);
        }
        KeyCmd::Revoke { id } => {
            let changed = keys::revoke(pool, None, id).await?;
            println!("{}", if changed { "revoked" } else { "not changed" });
        }
    }
    Ok(())
}

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
    /// Manage sending domains
    #[command(subcommand)]
    Domain(DomainCmd),
    /// Manage dashboard users
    #[command(subcommand)]
    User(UserCmd),
    /// Run the API and/or the background workers
    Serve {
        #[arg(long, value_enum, default_value_t = Role::All)]
        role: Role,
    },
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
enum DomainCmd {
    /// Add a domain to an account and print the DNS records to create
    Add {
        #[arg(long)]
        account: Uuid,
        name: String,
        #[arg(long)]
        tracking_subdomain: Option<String>,
    },
}

#[derive(Subcommand)]
enum UserCmd {
    /// Give someone dashboard access to an account (no email is sent). Safe to run again.
    Invite {
        email: String,
        #[arg(long)]
        account: Uuid,
        #[arg(long, value_enum, default_value_t = crate::users::MemberRole::Member)]
        role: crate::users::MemberRole,
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
        Command::User(UserCmd::Invite {
            email,
            account,
            role,
        }) => {
            accounts::get(&pool, account)
                .await?
                .ok_or_else(|| anyhow::anyhow!("no account {account}"))?;
            let (user, role) = crate::users::invite_direct(&pool, &email, account, role).await?;
            println!(
                "{} is {:?} of {account}. Log in at {}/login",
                user.email,
                role,
                config.web_base()
            );
        }
        Command::Domain(DomainCmd::Add {
            account,
            name,
            tracking_subdomain,
        }) => {
            let sender = sender::from_config(&config).await?;
            let state = crate::api::AppState::new(pool.clone(), config.clone(), sender);
            let acc = accounts::get(&pool, account)
                .await?
                .ok_or_else(|| anyhow::anyhow!("no account {account}"))?;
            let d = crate::domains::add(
                &state,
                &acc,
                crate::domains::AddDomain {
                    name,
                    tracking_subdomain,
                    open_tracking: false,
                    click_tracking: false,
                },
            )
            .await?;
            println!("domain {} ({:?})\nadd these DNS records:", d.name, d.status);
            for r in d.dns_records.iter() {
                println!(
                    "  {:<6} {:<45} {}   # {}",
                    r.record_type, r.name, r.value, r.purpose
                );
            }
        }

        Command::Serve { role } => {
            let sender = sender::from_config(&config).await?;
            let state = crate::api::AppState::new(pool, config, sender);
            if let Some(problem) = crate::system_mail::setup_problem(&state).await? {
                tracing::warn!(%problem, "dashboard login emails will not be sent until this is fixed");
            }
            let shutdown = tokio_util::sync::CancellationToken::new();
            let mut tasks = tokio::task::JoinSet::new();
            if role != Role::Worker {
                tasks.spawn(crate::api::serve(state.clone(), shutdown.clone()));
            }
            if role != Role::Api {
                crate::background::spawn_all(&mut tasks, state.clone(), shutdown.clone());
            }
            tokio::select! {
                _ = tokio::signal::ctrl_c() => tracing::info!("shutting down"),
                Some(res) = tasks.join_next() => {
                    tracing::error!(?res, "a task stopped early, shutting down");
                }
            }
            shutdown.cancel();
            while tasks.join_next().await.is_some() {}
        }
    }
    Ok(())
}

async fn account_cmd(pool: &sqlx::PgPool, config: &Config, cmd: AccountCmd) -> anyhow::Result<()> {
    match cmd {
        AccountCmd::Create { name } => {
            let name = name.trim();
            anyhow::ensure!(
                !name.is_empty() && name.len() <= 100,
                "name must be 1-100 characters"
            );
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
            println!(
                "{}",
                if changed {
                    "paused"
                } else {
                    "not changed (missing or already paused)"
                }
            );
        }
        AccountCmd::Resume { id } => {
            let changed = accounts::resume(pool, id).await?;
            println!(
                "{}",
                if changed {
                    "resumed"
                } else {
                    "not changed (missing or not paused)"
                }
            );
        }
        AccountCmd::SyncSes { id, all } => {
            let sender = sender::from_config(config).await?;
            let targets = match (id, all) {
                (Some(id), false) => vec![
                    accounts::get(pool, id)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("no account {id}"))?,
                ],
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
        KeyCmd::Create {
            account,
            name,
            access,
        } => {
            anyhow::ensure!(
                accounts::get(pool, account).await?.is_some(),
                "no account {account}"
            );
            let key = keys::create(pool, account, &name, access).await?;
            println!(
                "key id: {}\nkey:    {}\n(save it now, it is not shown again)",
                key.id, key.token
            );
        }
        KeyCmd::Revoke { id } => {
            let changed = keys::revoke(pool, None, id).await?;
            println!("{}", if changed { "revoked" } else { "not changed" });
        }
    }
    Ok(())
}

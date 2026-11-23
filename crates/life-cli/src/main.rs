//! Standalone, agent-friendly CLI + local MCP stdio transport.
mod action_spec;
mod reminder_runner;
mod remote_mcp;
use clap::{Parser, Subcommand};
use life_core::{Store, error_code};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, BufRead, Write},
    path::PathBuf,
};
#[derive(Parser)]
#[command(
    name = "life",
    version,
    about = "Life Manager — local-first CLI, independent of the desktop window"
)]
struct Cli {
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, default_value = "user")]
    actor: String,
    #[command(subcommand)]
    cmd: Cmd,
}
#[derive(Subcommand)]
enum Cmd {
    Today,
    Dashboard,
    Stats,
    #[command(subcommand)]
    Task(Resource),
    #[command(subcommand)]
    Project(Resource),
    #[command(subcommand)]
    Goal(Resource),
    #[command(subcommand)]
    Calendar(Calendar),
    #[command(subcommand)]
    Event(Resource),
    #[command(subcommand)]
    Habit(Habit),
    #[command(subcommand)]
    Note(Resource),
    #[command(subcommand)]
    Focus(Focus),
    #[command(subcommand)]
    Agent(Agent),
    #[command(subcommand)]
    Backup(Backup),
    Areas,
    #[command(subcommand)]
    Area(Resource),
    Search {
        query: String,
    },
    Export {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Call {
        action: String,
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        payload: Option<String>,
    },
    #[command(subcommand)]
    Mcp(Mcp),
    #[command(subcommand)]
    Db(Db),
    #[command(subcommand)]
    Reminder(Reminder),
}
#[derive(Subcommand)]
enum Reminder {
    /// Check and deliver due reminders through macOS Notification Center.
    Check,
    /// Preview due reminders without attempting delivery or claiming their lease.
    Preview,
    /// Register a per-user, once-per-minute launchd reminder worker.
    Install,
    /// Stop and remove the current user's launchd reminder worker.
    Uninstall,
    /// Check whether the reminder worker is installed and loaded.
    Status,
}
#[derive(Subcommand)]
enum Resource {
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        query: Option<String>,
    },
    Get {
        id: String,
    },
    Create {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        due: Option<String>,
        #[arg(long)]
        priority: Option<u8>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        area: Option<String>,
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        request_id: Option<String>,
    },
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        due: Option<String>,
        #[arg(long)]
        priority: Option<u8>,
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        expected_revision: Option<i64>,
    },
    Complete {
        id: String,
    },
    Delete {
        id: String,
    },
    Restore {
        id: String,
    },
}
#[derive(Subcommand)]
enum Calendar {
    List {
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
    },
    Conflicts {
        start_at: String,
        end_at: String,
    },
    Propose {
        #[arg(long)]
        date: Option<String>,
    },
}
#[derive(Subcommand)]
enum Habit {
    List,
    Create {
        title: String,
    },
    Log {
        habit_id: String,
        #[arg(long)]
        day: Option<String>,
        #[arg(long, default_value_t = 1)]
        value: i64,
    },
    Delete {
        id: String,
    },
}
#[derive(Subcommand)]
enum Focus {
    List,
    Start {
        #[arg(long)]
        task: Option<String>,
    },
    Stop {
        id: String,
    },
}
#[derive(Subcommand)]
enum Agent {
    History,
}
#[derive(Subcommand)]
enum Backup {
    Create {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Verify {
        path: PathBuf,
    },
    Restore {
        path: PathBuf,
        #[arg(long)]
        confirm: bool,
    },
}
/// Manage the local database and its protection.
#[derive(Subcommand)]
enum Db {
    /// Show how the database is protected.
    Status,
    /// Encrypt the database with a passphrase, after a verified backup.
    Encrypt {
        /// The passphrase. Read from LIFE_DB_PASSPHRASE when it is not given.
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Decrypt the database back to a plain SQLite file, after a verified backup.
    Decrypt {
        /// The passphrase of the encrypted database.
        #[arg(long)]
        passphrase: Option<String>,
        /// Must be set. The database becomes readable on disk.
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand)]
enum Mcp {
    Serve {
        #[arg(long)]
        stdio: bool,
    },
    Tools,
    /// Serve the MCP protocol over HTTP on the loopback interface.
    Remote {
        /// The port. A free high port is chosen when this is zero.
        #[arg(long, default_value_t = 0)]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Required to bind anything other than the loopback interface.
        #[arg(long)]
        allow_remote: bool,
    },
}
fn main() {
    let cli = Cli::parse();
    let result = run(cli);
    if let Err(e) = result {
        let o = json!({"ok":false,"error":{"code":error_code(&e),"message":e.to_string()}});
        eprintln!("{}", o);
        std::process::exit(1)
    }
}
fn parse_file(path: Option<PathBuf>) -> life_core::Result<Value> {
    match path {
        Some(p) => Ok(serde_json::from_slice(&fs::read(p)?)?),
        None => Ok(json!({})),
    }
}
fn write_fields(p: &mut Value, fields: Vec<(&str, Option<String>)>) {
    for (k, v) in fields {
        if let Some(v) = v {
            p[k] = json!(v)
        }
    }
}
fn resource(noun: &str, cmd: Resource) -> life_core::Result<(String, Value)> {
    let mut p = json!({});
    let suffix = match cmd {
        Resource::List {
            status,
            project,
            query,
        } => {
            write_fields(
                &mut p,
                vec![
                    ("status", status),
                    ("project_id", project),
                    ("query", query),
                ],
            );
            "list"
        }
        Resource::Get { id } => {
            p["id"] = json!(id);
            "get"
        }
        Resource::Create {
            title,
            name,
            description,
            due,
            priority,
            project,
            area,
            input,
            request_id,
        } => {
            p = parse_file(input)?;
            write_fields(
                &mut p,
                vec![
                    ("title", title),
                    ("name", name),
                    ("description", description),
                    ("due_date", due),
                    ("project_id", project),
                    ("area_id", area),
                    ("request_id", request_id),
                ],
            );
            if let Some(q) = priority {
                p["priority"] = json!(q)
            }
            "create"
        }
        Resource::Update {
            id,
            title,
            name,
            description,
            status,
            due,
            priority,
            input,
            expected_revision,
        } => {
            p = parse_file(input)?;
            p["id"] = json!(id);
            write_fields(
                &mut p,
                vec![
                    ("title", title),
                    ("name", name),
                    ("description", description),
                    ("status", status),
                    ("due_date", due),
                ],
            );
            if let Some(q) = priority {
                p["priority"] = json!(q)
            }
            if let Some(r) = expected_revision {
                p["expected_revision"] = json!(r)
            }
            "update"
        }
        Resource::Complete { id } => {
            p["id"] = json!(id);
            "complete"
        }
        Resource::Delete { id } => {
            p["id"] = json!(id);
            "delete"
        }
        Resource::Restore { id } => {
            p["id"] = json!(id);
            "restore"
        }
    };
    Ok((format!("{noun}.{suffix}"), p))
}
fn run(cli: Cli) -> life_core::Result<()> {
    if let Cmd::Mcp(Mcp::Serve { .. }) = &cli.cmd {
        return serve_mcp(cli.db);
    }
    if let Cmd::Mcp(Mcp::Remote {
        port,
        host,
        allow_remote,
    }) = &cli.cmd
    {
        // The server is local by default. Binding another interface needs an
        // explicit flag, because the transport is plain HTTP.
        let loopback = host == "127.0.0.1" || host == "localhost" || host == "::1";
        if !loopback && !*allow_remote {
            return Err(life_core::LifeError::Invalid(format!(
                "Refusing to bind {host}. The remote server speaks plain HTTP. \
                 Use --allow-remote only when you understand the exposure."
            )));
        }
        let store = match &cli.db {
            Some(path) => Store::open(path)?,
            None => Store::default()?,
        };
        return remote_mcp::serve(store, host, *port);
    }
    if let Cmd::Db(cmd) = &cli.cmd {
        // The store may be encrypted, so the key is resolved before the store
        // is opened. Without it the database cannot be opened at all.
        let path = match &cli.db {
            Some(path) => path.clone(),
            None => life_core::default_db_path(),
        };
        let secret = match cmd {
            Db::Decrypt { passphrase, .. } => passphrase
                .clone()
                .or_else(|| std::env::var("LIFE_DB_PASSPHRASE").ok()),
            _ => std::env::var("LIFE_DB_PASSPHRASE").ok(),
        };
        let mut store = match secret {
            Some(value) => {
                Store::open_with_passphrase(&path, &value).or_else(|_| Store::open(&path))?
            }
            None => Store::open(&path)?,
        };
        let result = match cmd {
            Db::Status => store.call("encryption.status", &json!({}))?,
            Db::Encrypt { passphrase } => {
                let secret = match passphrase {
                    Some(value) => value.clone(),
                    None => std::env::var("LIFE_DB_PASSPHRASE").map_err(|_| {
                        life_core::LifeError::Invalid(
                            "Pass the passphrase with --passphrase, or set LIFE_DB_PASSPHRASE."
                                .into(),
                        )
                    })?,
                };
                store.call(
                    "encryption.enable",
                    &json!({"mode":"passphrase","passphrase":secret}),
                )?
            }
            Db::Decrypt {
                passphrase,
                confirm,
            } => {
                if !*confirm {
                    return Err(life_core::LifeError::Invalid(
                        "Set --confirm to decrypt. The database becomes readable on disk.".into(),
                    ));
                }
                // The store may already be open with the key, but the command
                // must also work when the database is opened without it, so
                // the passphrase is resolved before the store is used.
                let secret = match passphrase {
                    Some(value) => value.clone(),
                    None => std::env::var("LIFE_DB_PASSPHRASE").map_err(|_| {
                        life_core::LifeError::Invalid(
                            "Pass the passphrase with --passphrase, or set LIFE_DB_PASSPHRASE."
                                .into(),
                        )
                    })?,
                };
                let _ = &secret;
                let mut payload = json!({"confirm":true,"passphrase":secret});
                payload["actor"] = json!("user");
                store.call("encryption.disable", &payload)?
            }
        };
        if cli.json {
            println!("{}", json!({"ok":true,"data":result}))
        } else {
            println!("{}", serde_json::to_string_pretty(&result)?)
        }
        return Ok(());
    }
    if let Cmd::Mcp(Mcp::Tools) = &cli.cmd {
        println!("{}", serde_json::to_string_pretty(&mcp_tools())?);
        return Ok(());
    }
    if let Cmd::Reminder(cmd) = &cli.cmd {
        let result = match cmd {
            Reminder::Check => reminder_runner::check(cli.db.as_deref(), false)?,
            Reminder::Preview => reminder_runner::check(cli.db.as_deref(), true)?,
            Reminder::Install => reminder_runner::install(cli.db.as_deref())?,
            Reminder::Uninstall => reminder_runner::uninstall()?,
            Reminder::Status => reminder_runner::status()?,
        };
        if cli.json {
            println!("{}", json!({"ok":true,"data":result}))
        } else {
            println!("{}", serde_json::to_string_pretty(&result)?)
        }
        return Ok(());
    }
    let mut store = if let Some(path) = cli.db {
        Store::open(path)?
    } else {
        Store::default()?
    };
    let actor = cli.actor;
    let (action, mut p) = match cli.cmd {
        Cmd::Db(_) => {
            unreachable!("the db subcommand is handled before the action dispatch")
        }
        Cmd::Today => ("today".into(), json!({})),
        Cmd::Dashboard => ("dashboard".into(), json!({})),
        Cmd::Stats => ("stats".into(), json!({})),
        Cmd::Areas => ("areas.list".into(), json!({})),
        Cmd::Area(c) => resource("areas", c)?,
        Cmd::Task(c) => resource("tasks", c)?,
        Cmd::Project(c) => resource("projects", c)?,
        Cmd::Goal(c) => resource("goals", c)?,
        Cmd::Note(c) => resource("notes", c)?,
        Cmd::Event(c) => resource("events", c)?,
        Cmd::Calendar(Calendar::List { from, to }) => {
            let mut p = json!({});
            write_fields(&mut p, vec![("from", from), ("to", to)]);
            ("events.list".into(), p)
        }
        Cmd::Calendar(Calendar::Conflicts { start_at, end_at }) => (
            "calendar.conflicts".into(),
            json!({"start_at":start_at,"end_at":end_at}),
        ),
        Cmd::Calendar(Calendar::Propose { date }) => {
            ("schedule.propose".into(), json!({"date":date}))
        }
        Cmd::Habit(Habit::List) => ("habits.list".into(), json!({})),
        Cmd::Habit(Habit::Create { title }) => ("habits.create".into(), json!({"title":title})),
        Cmd::Habit(Habit::Log {
            habit_id,
            day,
            value,
        }) => (
            "habits.log".into(),
            json!({"habit_id":habit_id,"day":day,"value":value}),
        ),
        Cmd::Habit(Habit::Delete { id }) => ("habits.delete".into(), json!({"id":id})),
        Cmd::Focus(Focus::List) => ("focus.list".into(), json!({})),
        Cmd::Focus(Focus::Start { task }) => ("focus.start".into(), json!({"task_id":task})),
        Cmd::Focus(Focus::Stop { id }) => ("focus.stop".into(), json!({"id":id})),
        Cmd::Agent(Agent::History) => ("history.list".into(), json!({})),
        Cmd::Backup(Backup::Create { out }) => ("backup.create".into(), json!({"path":out})),
        Cmd::Backup(Backup::Verify { path }) => ("backup.verify".into(), json!({"path":path})),
        Cmd::Backup(Backup::Restore { path, confirm }) => (
            "backup.restore".into(),
            json!({"path":path,"confirm":confirm}),
        ),
        Cmd::Search { query } => ("search".into(), json!({"query":query})),
        Cmd::Export { out } => {
            let result = store.call("export", &json!({}))?;
            if let Some(path) = out {
                fs::write(&path, serde_json::to_vec_pretty(&result)?)?;
                println!("{}", path.display())
            } else {
                println!("{}", serde_json::to_string_pretty(&result)?)
            }
            return Ok(());
        }
        Cmd::Call {
            action,
            input,
            payload,
        } => {
            let p = if let Some(payload) = payload {
                serde_json::from_str(&payload)?
            } else {
                parse_file(input)?
            };
            (action, p)
        }
        Cmd::Mcp(_) | Cmd::Reminder(_) => unreachable!(),
    };
    p["actor"] = json!(actor);
    let result = store.call(&action, &p)?;
    if cli.json {
        println!("{}", json!({"ok":true,"data":result}))
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?)
    }
    Ok(())
}
fn mcp_tools() -> Value {
    json!(action_spec::tools())
}

fn serve_mcp(db: Option<PathBuf>) -> life_core::Result<()> {
    let mut store = if let Some(path) = db {
        Store::open(path)?
    } else {
        Store::default()?
    };
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let id = req.get("id").cloned();
        let method = req["method"].as_str().unwrap_or("");
        if id.is_none() {
            continue;
        }
        let reply = match method {
            "initialize" => {
                json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"life-manager","version":"0.1.0"}})
            }
            "ping" => json!({}),
            "tools/list" => json!({"tools":mcp_tools()}),
            "tools/call" => {
                let n = req["params"]["name"].as_str().unwrap_or("");
                let Some(action) = action_spec::action_for_tool(n) else {
                    return Err(life_core::LifeError::Invalid(format!("Unknown tool {n}")));
                };
                let mut p = req["params"]["arguments"].clone();
                if !p.is_object() {
                    p = json!({})
                }
                if let Err(message) = action_spec::validate(action, &p) {
                    json!({"isError":true,"content":[{"type":"text","text":json!({"error":{"code":"VALIDATION_ERROR","message":message}}).to_string()}]})
                } else {
                    p["actor"] = json!("mcp");
                    match store.call(action, &p) {
                        Ok(v) => json!({"content":[{"type":"text","text":v.to_string()}]}),
                        Err(e) => {
                            json!({"isError":true,"content":[{"type":"text","text":json!({"error":{"code":error_code(&e),"message":e.to_string()}}).to_string()}]})
                        }
                    }
                }
            }
            _ => json!({"error":{"code":-32601,"message":"Method not found"}}),
        };
        let envelope = if reply.get("error").is_some() {
            json!({"jsonrpc":"2.0","id":id,"error":reply["error"]})
        } else {
            json!({"jsonrpc":"2.0","id":id,"result":reply})
        };
        writeln!(stdout, "{}", envelope)?;
        stdout.flush()?;
    }
    Ok(())
}

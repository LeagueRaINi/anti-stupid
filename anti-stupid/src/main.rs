use std::io::{self, Write};
use std::path::PathBuf;
use std::{fs, process};

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sysinfo::{Process, ProcessExt, System, SystemExt};

#[derive(Serialize, Deserialize)]
struct Config {
    command: String,
    #[serde(with = "serde_regex")]
    watchlist: Vec<Regex>,
}

#[derive(Parser, Debug)]
#[clap(about, author, version)]
struct Args {
    #[cfg_attr(debug_assertions, structopt(default_value = "./config.json"))]
    cfg_path: PathBuf,
}

fn main() -> Result<()> {
    let Args { cfg_path } = Args::parse();

    let cfg = fs::read(cfg_path).context("Failed to read config.json")?;
    let cfg = serde_json::from_str::<Config>(
        std::str::from_utf8(&cfg).context("Failed to convert config.json content to string")?,
    )
    .context("Failed to parse config.json")?;

    let mut sys = System::new_all();

    sys.refresh_all();

    let proc_list = sys.processes().iter().filter_map(|(_, p)| {
        (cfg.watchlist.iter().any(|wp| wp.is_match(p.name())) || matches!(p.exe().file_name(), Some(name) if cfg.watchlist.iter().any(|wp| wp.is_match(&name.to_string_lossy()))))
            .then_some(p)
    }).collect::<Vec<&Process>>();

    if proc_list.is_empty() {
        let output = process::Command::new("cmd")
            .arg("/c")
            .arg(&cfg.command)
            .output()
            .context("Failed to run command")?;

        println!("Command status: {}, output:", output.status);
        io::stdout().write_all(&output.stdout).context("Failed to write to stdout")?;
        io::stderr().write_all(&output.stderr).context("Failed to write to stderr")?;
    } else {
        // TODO!: kill processes?
        for proc in proc_list {
            println!("[{:05}] {} ({:?}) is still running!", proc.pid(), proc.name(), proc.exe());
        }
        // TODO!: retry after keypress?
        io::stdin().read_line(&mut String::new()).unwrap();
    }

    Ok(())
}

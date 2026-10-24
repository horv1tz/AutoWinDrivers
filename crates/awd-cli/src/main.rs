//! `awd` — the AutoWinDrivers command-line interface.

mod render;

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, bail};
use awd_core::{HostInfo, Lang, ProblemCode, ScanOptions, ScanReport};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "awd",
    version,
    about = "AutoWinDrivers: scan hardware and manage its drivers"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Enumerate devices and show their driver status.
    Scan(ScanArgs),
    /// Explain a Device Manager problem code, e.g. `awd problem 28`.
    Problem {
        /// The numeric problem code.
        code: u32,
        /// Language of the explanation.
        #[arg(long, value_enum, default_value_t = LangArg::En)]
        lang: LangArg,
    },
}

#[derive(clap::Args, Debug)]
struct ScanArgs {
    /// Print the full report as JSON instead of a table.
    #[arg(long)]
    json: bool,
    /// Include devices that are known but not currently connected.
    #[arg(long)]
    all: bool,
    /// Show only devices with a problem (including disabled ones).
    #[arg(long)]
    problems: bool,
    /// Show only devices of this setup class (e.g. `Display`, `Net`, `USB`).
    #[arg(long)]
    class: Option<String>,
    /// Render a previously saved JSON report instead of scanning this machine.
    #[arg(long, value_name = "FILE")]
    input: Option<PathBuf>,
    /// Save the full JSON report to this file (in addition to printing).
    #[arg(long, short, value_name = "FILE")]
    output: Option<PathBuf>,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum LangArg {
    En,
    Ru,
}

impl From<LangArg> for Lang {
    fn from(l: LangArg) -> Self {
        match l {
            LangArg::En => Lang::En,
            LangArg::Ru => Lang::Ru,
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Scan(args) => scan(args),
        Command::Problem { code, lang } => {
            let p = ProblemCode(code);
            println!("{}", render::problem(p, lang.into()));
            Ok(())
        }
    }
}

fn scan(args: ScanArgs) -> anyhow::Result<()> {
    let report = match &args.input {
        Some(path) => {
            let text = fs::read_to_string(path)
                .with_context(|| format!("cannot read {}", path.display()))?;
            serde_json::from_str::<ScanReport>(&text)
                .with_context(|| format!("{} is not a scan report", path.display()))?
        }
        None => {
            let Some(source) = awd_win::default_device_source() else {
                bail!(
                    "live scanning is only available on Windows; \
                     use --input <report.json> to render a saved report"
                );
            };
            let devices = source
                .scan(&ScanOptions {
                    include_not_present: args.all,
                })
                .context("device scan failed")?;
            ScanReport::new(HostInfo::current(), devices)
        }
    };

    if let Some(path) = &args.output {
        let json = serde_json::to_string_pretty(&report)?;
        fs::write(path, json).with_context(|| format!("cannot write {}", path.display()))?;
    }

    let filter = render::Filter {
        problems_only: args.problems,
        class: args.class.as_deref(),
    };
    let mut out = std::io::stdout().lock();
    if args.json {
        let filtered = render::filtered(&report, &filter);
        serde_json::to_writer_pretty(&mut out, &filtered)?;
        writeln!(out)?;
    } else {
        out.write_all(render::table(&report, &filter).as_bytes())?;
    }
    Ok(())
}

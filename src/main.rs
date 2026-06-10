mod config;
mod model;
#[cfg(feature = "overlay")]
mod overlay;
mod providers;
mod render;
mod snapshot;
mod windows;

use std::thread;
use std::time::Duration;

use chrono::Utc;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "limitbar",
    version,
    about = "Always-visible usage meter for LLM rate-limit windows"
)]
struct Cli {
    /// Refresh in the terminal every N seconds instead of printing once
    #[arg(short = 'w', long = "watch", value_name = "secs")]
    watch: Option<u64>,

    /// Show a floating always-on-top overlay (requires the `overlay` feature)
    #[arg(long = "overlay")]
    overlay: bool,
}

fn main() {
    let cli = Cli::parse();

    let config = match config::load() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("limitbar: {err}");
            std::process::exit(1);
        }
    };

    if cli.overlay {
        run_overlay(config);
        return;
    }

    if let Some(secs) = cli.watch {
        let interval = Duration::from_secs(secs.max(1));
        loop {
            print!("\x1b[2J\x1b[H");
            print_snapshot(&config);
            thread::sleep(interval);
        }
    }

    print_snapshot(&config);
}

fn print_snapshot(config: &config::Config) {
    let now = Utc::now();
    let (usages, warnings) = snapshot::collect(config, now);

    for usage in &usages {
        println!("{}", render::render_line(usage, now));
    }
    for warning in &warnings {
        eprintln!("limitbar: warning: {warning}");
    }
    if usages.is_empty() {
        eprintln!("limitbar: no providers produced data");
        std::process::exit(1);
    }
}

#[cfg(feature = "overlay")]
fn run_overlay(config: config::Config) {
    if let Err(err) = overlay::run(config) {
        eprintln!("limitbar: overlay failed: {err}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "overlay"))]
fn run_overlay(_config: config::Config) {
    eprintln!(
        "limitbar: this build has no overlay support; reinstall with `cargo install limitbar --features overlay`"
    );
    std::process::exit(1);
}

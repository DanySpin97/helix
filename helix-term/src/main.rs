#![allow(unused)]

mod application;
mod commands;
mod compositor;
mod keymap;
mod ui;

use application::Application;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Clap;

use anyhow::Error;

fn setup_logging(verbosity: u32) -> Result<()> {
    let mut base_config = fern::Dispatch::new();

    // Let's say we depend on something which whose "info" level messages are too
    // verbose to include in end-user output. If we don't need them,
    // let's not include them.
    // .level_for("overly-verbose-target", log::LevelFilter::Warn)

    base_config = match verbosity {
        0 => base_config.level(log::LevelFilter::Warn),
        1 => base_config.level(log::LevelFilter::Info),
        2 => base_config.level(log::LevelFilter::Debug),
        _3_or_more => base_config.level(log::LevelFilter::Trace),
    };

    let home = dirs_next::home_dir().context("can't find the home directory")?;

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} [{}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file(home.join("helix.log"))?);

    base_config.chain(file_config).apply()?;

    Ok(())
}

#[derive(Clap)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Opts {
    #[clap(short, long, parse(from_occurrences))]
    verbose: u32,
    #[clap(short = 'V', long)]
    version: bool,
    files: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    setup_logging(opts.verbose).context("failed to initialize logging.")?;

    // initialize language registry
    use helix_core::config_dir;
    use helix_core::syntax::{Loader, LOADER};

    // load $HOME/.config/helix/languages.toml, fallback to default config
    let config = std::fs::read(config_dir().join("languages.toml"));
    let toml = config
        .as_deref()
        .unwrap_or(include_bytes!("../../languages.toml"));

    let config = toml::from_slice(toml).context("Could not parse languages.toml")?;
    LOADER.get_or_init(|| Loader::new(config));

    let runtime = tokio::runtime::Runtime::new().context("unable to start tokio runtime")?;

    // TODO: use the thread local executor to spawn the application task separately from the work pool
    let mut app = Application::new(opts.files).context("unable to create new appliction")?;
    runtime.block_on(async move {
        app.run().await;
    });

    Ok(())
}

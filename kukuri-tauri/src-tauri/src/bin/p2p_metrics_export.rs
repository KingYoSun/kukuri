use anyhow::{Context, Result};
use kukuri_lib::ops::p2p::metrics;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn usage() -> &'static str {
    "Usage: p2p_metrics_export [--output <path>] [--pretty]"
}

fn write_output(path: &Path, data: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }
    fs::write(path, data).with_context(|| format!("Failed to write {}", path.display()))
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut output: Option<PathBuf> = None;
    let mut pretty = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--output requires a path\n{}", usage()))?;
                output = Some(PathBuf::from(path));
            }
            "--pretty" => {
                pretty = true;
            }
            "-h" | "--help" => {
                println!("{}", usage());
                return Ok(());
            }
            other => {
                return Err(anyhow::anyhow!(format!(
                    "Unknown argument: {other}\n{}",
                    usage()
                )));
            }
        }
    }

    let snapshot = metrics::snapshot_full();
    let payload = if pretty {
        serde_json::to_string_pretty(&snapshot)?
    } else {
        serde_json::to_string(&snapshot)?
    };

    if let Some(path) = output {
        write_output(&path, &payload)?;
        println!("Metrics written to {}", path.display());
    } else {
        println!("{payload}");
    }

    Ok(())
}

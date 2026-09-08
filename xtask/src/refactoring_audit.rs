use std::process::Command;

use anyhow::{Context, Result, bail};

/// Run the same small, read-only evaluator used by the scheduled workflow.
/// Invoke the interpreter directly, including on Windows: reasons are arguments,
/// never shell command text. Do not use run(), which writes timing to stdout.
pub(crate) fn refactoring_audit_check(args: impl Iterator<Item = String>) -> Result<()> {
    let python = std::env::var("KUKURI_AUDIT_PYTHON")
        .unwrap_or_else(|_| if cfg!(windows) { "python" } else { "python3" }.to_string());
    let status = Command::new(python)
        .arg(crate::root_dir().join("scripts/refactoring-audit/check.py"))
        .args(args)
        .status()
        .context("failed to run audit checker; see docs/runbooks/dev.md for Python setup")?;
    if !status.success() {
        bail!("refactoring-audit-check failed: {status}");
    }
    Ok(())
}

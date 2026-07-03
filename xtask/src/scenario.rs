use anyhow::{Context, Result};

#[allow(unused_imports)]
use crate::*;

pub(crate) fn e2e_smoke(name: &str) -> Result<()> {
    run_timed_step(format!("scenario {name}"), || {
        let root = root_dir();
        let artifacts = artifacts_dir(name);
        let name = name.to_string();
        let handle = std::thread::Builder::new()
            .name(format!("scenario-{name}"))
            .stack_size(64 * 1024 * 1024)
            .spawn(move || -> Result<_> {
                let runtime =
                    tokio::runtime::Runtime::new().context("failed to build tokio runtime")?;
                runtime.block_on(kukuri_harness::run_named_scenario(
                    &root,
                    name.as_str(),
                    &artifacts,
                ))
            })
            .context("failed to spawn scenario runner thread")?;
        let result = handle
            .join()
            .map_err(|_| anyhow::anyhow!("scenario runner thread panicked"))??;
        let metrics = kukuri_harness::summarize_metrics(&result);
        for (key, value) in metrics {
            println!("{key}={value}");
        }
        Ok(())
    })
}

pub(crate) fn scenario(name: &str) -> Result<()> {
    if scenario_requires_cn_postgres(name) {
        with_cn_postgres(|| e2e_smoke(name))
    } else {
        e2e_smoke(name)
    }
}

pub(crate) fn scenario_requires_cn_postgres(name: &str) -> bool {
    if std::env::var_os("KUKURI_HARNESS_COMMUNITY_NODE_BASE_URL").is_some() {
        return false;
    }
    matches!(
        name,
        "community_node_public_connectivity" | "community_node_multi_device_connectivity"
    )
}

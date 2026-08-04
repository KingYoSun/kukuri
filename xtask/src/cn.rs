use anyhow::Result;

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cn_check() -> Result<()> {
    let packages = cn_packages()?;
    let mut clippy_args = cargo_package_args("clippy", &packages);
    clippy_args.push("--all-targets".to_string());
    clippy_args.push("--".to_string());
    clippy_args.push("-D".to_string());
    clippy_args.push("warnings".to_string());
    run("cargo", clippy_args, &root_dir())
}

pub(crate) fn cn_test() -> Result<()> {
    let packages = cn_packages()?;
    with_cn_postgres(|| {
        run_with_owned_env(
            "cargo",
            cargo_package_args("test", &packages),
            &root_dir(),
            &cn_test_envs(),
        )
    })
}

pub(crate) fn cn_compose_envs() -> [(&'static str, &'static str); 4] {
    [
        ("CN_POSTGRES_PASSWORD", "cn_password"),
        (
            "COMMUNITY_NODE_JWT_SECRET",
            "xtask-test-secret-0123456789abcdef",
        ),
        // #615 で compose に加わった必須 interpolation（cn-arcadedb / cn-indexer 系）。
        // 起動するのは cn-postgres / cn-valkey だけでも、compose は file 全体を
        // interpolate するため未設定だと `up` 自体が失敗する。
        ("CN_ARCADEDB_ROOT_PASSWORD", "cn_arcadedb_password"),
        (
            "COMMUNITY_NODE_CHANNEL_SECRET_KEY",
            "xtask-channel-secret-key-0123456789abcdef",
        ),
    ]
}

pub(crate) fn cn_test_envs() -> Vec<(String, String)> {
    vec![
        (
            "KUKURI_CN_RUN_INTEGRATION_TESTS".to_string(),
            "1".to_string(),
        ),
        (
            "COMMUNITY_NODE_DATABASE_URL".to_string(),
            cn_test_database_url(),
        ),
        (
            "COMMUNITY_NODE_RENDEZVOUS_REDIS_URL".to_string(),
            cn_test_rendezvous_redis_url(),
        ),
    ]
}

pub(crate) fn cn_test_database_url() -> String {
    cn_test_database_url_for(std::env::var("CN_POSTGRES_PORT").ok())
}

pub(crate) fn cn_test_database_url_for(port_env: Option<String>) -> String {
    let port = resolve_port(port_env, "15432");
    format!("postgres://cn:cn_password@127.0.0.1:{port}/cn")
}

pub(crate) fn cn_test_rendezvous_redis_url() -> String {
    cn_test_rendezvous_redis_url_for(std::env::var("CN_VALKEY_PORT").ok())
}

pub(crate) fn cn_test_rendezvous_redis_url_for(port_env: Option<String>) -> String {
    let port = resolve_port(port_env, "16379");
    format!("redis://127.0.0.1:{port}/")
}

fn resolve_port(port_env: Option<String>, default: &str) -> String {
    port_env
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

pub(crate) fn with_cn_postgres<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    let root = root_dir();
    // 15432 / 16379 は Linux の一般的な ephemeral port range (32768-60999) 外に置き、
    // outgoing connection の一時ポートとの衝突を避ける。
    // さらに起動前に残骸スタックを掃除しておく。前回の cn-test が異常終了して compose スタックが
    // 残っていると固定ホストポートが専有されたままになり、`up` が "address already in use" で
    // 失敗する。pre-up の down は冪等で、残骸が無ければ no-op。
    // docker 未起動などの環境異常では失敗し得るが、その場合は後続の `up` が本来のエラーを
    // 返すため、ここでは best-effort（結果を無視）にする。
    let _ = run_with_env(
        "docker",
        [
            "compose",
            "-f",
            "docker-compose.community-node.yml",
            "down",
            "-v",
            "--remove-orphans",
        ],
        &root,
        &cn_compose_envs(),
    );
    run_with_env(
        "docker",
        [
            "compose",
            "-f",
            "docker-compose.community-node.yml",
            "up",
            "-d",
            "--wait",
            "cn-postgres",
            "cn-valkey",
        ],
        &root,
        &cn_compose_envs(),
    )?;
    let operation_result = operation();
    let shutdown_result = run_with_env(
        "docker",
        [
            "compose",
            "-f",
            "docker-compose.community-node.yml",
            "down",
            "-v",
            "--remove-orphans",
        ],
        &root,
        &cn_compose_envs(),
    );
    match (operation_result, shutdown_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(operation_error), Err(shutdown_error)) => Err(operation_error.context(format!(
            "failed to tear down cn-postgres after error: {shutdown_error:#}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cn_test_urls_use_documented_default_ports() {
        assert_eq!(
            cn_test_database_url_for(None),
            "postgres://cn:cn_password@127.0.0.1:15432/cn"
        );
        assert_eq!(
            cn_test_rendezvous_redis_url_for(None),
            "redis://127.0.0.1:16379/"
        );
    }

    #[test]
    fn cn_test_urls_respect_port_overrides_and_ignore_blank_values() {
        assert_eq!(
            cn_test_database_url_for(Some(" 25432 ".to_string())),
            "postgres://cn:cn_password@127.0.0.1:25432/cn"
        );
        assert_eq!(
            cn_test_rendezvous_redis_url_for(Some(String::new())),
            "redis://127.0.0.1:16379/"
        );
    }
}

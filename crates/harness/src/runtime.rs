use crate::*;

const DEFAULT_CN_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:55432/cn";

pub(crate) struct ScenarioRuntime {
    pub(crate) db_path: PathBuf,
    pub(crate) network: FakeNetwork,
    pub(crate) app: Option<AppService>,
    pub(crate) current_topic: Option<String>,
    pub(crate) current_channel_id: Option<String>,
    pub(crate) private_channels: BTreeMap<String, String>,
}

impl ScenarioRuntime {
    pub(crate) async fn launch(&mut self) -> Result<()> {
        let store = Arc::new(
            SqliteStore::connect_file(&self.db_path)
                .await
                .with_context(|| {
                    format!("failed to open scenario db {}", self.db_path.display())
                })?,
        );
        let transport = Arc::new(FakeTransport::new("desktop", self.network.clone()));
        self.app = Some(AppService::new(store, transport));
        Ok(())
    }

    pub(crate) fn app(&self) -> Result<&AppService> {
        self.app
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("desktop app is not running"))
    }

    pub(crate) fn topic_or_default(&self, default_topic: &str) -> String {
        self.current_topic
            .clone()
            .unwrap_or_else(|| default_topic.to_string())
    }

    pub(crate) fn current_scope(&self) -> TimelineScope {
        match self.current_channel_id.as_ref() {
            Some(channel_id) => TimelineScope::Channel {
                channel_id: ChannelId::new(channel_id.clone()),
            },
            None => TimelineScope::Public,
        }
    }
}

pub(crate) struct CommunityNodeStack {
    pub(crate) database: TestDatabase,
    pub(crate) user_api_task: tokio::task::JoinHandle<()>,
    pub(crate) _iroh_relay: SpawnedIrohRelay,
    pub(crate) base_url: String,
    pub(crate) iroh_relay_url: String,
}

impl CommunityNodeStack {
    pub(crate) async fn spawn(prefix: &str) -> Result<Self> {
        let admin_database_url = community_node_admin_database_url();
        let database = TestDatabase::create(admin_database_url.as_str(), prefix).await?;
        let iroh_relay = kukuri_cn_iroh_relay::spawn_server(IrohRelayConfig {
            http_bind_addr: "127.0.0.1:0"
                .parse()
                .expect("valid loopback relay bind addr"),
            tls: None,
        })
        .await?;
        let iroh_relay_url = format!("http://{}", iroh_relay.http_addr());

        let user_api_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind community-node user-api listener")?;
        let user_api_addr = user_api_listener.local_addr()?;
        let base_url = format!("http://{user_api_addr}");

        let user_api_state = build_user_api_state(&UserApiConfig {
            bind_addr: user_api_addr,
            database_url: database.database_url.clone(),
            base_url: base_url.clone(),
            public_base_url: base_url.clone(),
            connectivity_urls: vec![iroh_relay_url.clone()],
            jwt_config: JwtConfig::new("kukuri-cn-harness", "test-secret", 3600),
        })
        .await
        .context("failed to build community-node user-api state")?;
        let user_api_task = tokio::spawn(async move {
            axum::serve(
                user_api_listener,
                user_api_app_router(user_api_state)
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("community-node user-api server");
        });

        Ok(Self {
            database,
            user_api_task,
            _iroh_relay: iroh_relay,
            base_url,
            iroh_relay_url,
        })
    }

    pub(crate) async fn shutdown(self) -> Result<()> {
        self.user_api_task.abort();
        self.database.cleanup().await
    }
}

pub(crate) async fn shutdown_runtime(runtime: DesktopRuntime, label: &str) -> Result<()> {
    timeout(Duration::from_secs(30), runtime.shutdown())
        .await
        .with_context(|| format!("timed out waiting for {label}"))?;
    Ok(())
}

pub(crate) fn community_node_admin_database_url() -> String {
    std::env::var("COMMUNITY_NODE_DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CN_ADMIN_DATABASE_URL.to_string())
}

pub(crate) fn persist_runtime_identity(db_path: &Path, keys: &KukuriKeys) -> Result<()> {
    std::fs::write(
        db_path.with_extension("identity-key"),
        keys.export_secret_hex(),
    )
    .with_context(|| format!("failed to seed identity for {}", db_path.display()))
}

pub(crate) fn cleanup_runtime_artifacts(db_path: &Path) -> Result<()> {
    let config_paths = [
        db_path.to_path_buf(),
        db_path.with_extension("db-shm"),
        db_path.with_extension("db-wal"),
        db_path.with_extension("iroh-data"),
        db_path.with_extension("community-node.json"),
        db_path.with_extension("identity-store"),
        db_path.with_extension("identity-key"),
        db_path.with_extension("nsec"),
    ];
    for path in config_paths {
        if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("failed to remove stale directory {}", path.display()))?;
        } else if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove stale file {}", path.display()))?;
        }
    }
    if let (Some(parent), Some(stem)) = (db_path.parent(), db_path.file_stem()) {
        let stem = stem.to_string_lossy();
        let optional_secret_prefixes = [
            format!("{stem}.private-channel-capabilities-"),
            format!("{stem}.community-node-token-"),
        ];
        for entry in std::fs::read_dir(parent)
            .with_context(|| format!("failed to read {}", parent.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if optional_secret_prefixes
                .iter()
                .any(|prefix| file_name.starts_with(prefix))
            {
                if path.is_dir() {
                    std::fs::remove_dir_all(&path).with_context(|| {
                        format!("failed to remove stale directory {}", path.display())
                    })?;
                } else if path.exists() {
                    std::fs::remove_file(&path).with_context(|| {
                        format!("failed to remove stale file {}", path.display())
                    })?;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn remove_sqlite_runtime_db(db_path: &Path) -> Result<()> {
    for path in [
        db_path.to_path_buf(),
        db_path.with_extension("db-shm"),
        db_path.with_extension("db-wal"),
    ] {
        if !path.exists() {
            continue;
        }
        std::fs::remove_file(&path)
            .with_context(|| format!("failed to remove sqlite artifact {}", path.display()))?;
    }
    Ok(())
}

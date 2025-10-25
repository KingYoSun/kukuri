# Phase 5 OfflineService Adapter 險ｭ險・譛邨よ峩譁ｰ譌･: 2025蟷ｴ10譛・5譌･

## 閭梧勹
- `application::services::OfflineService` 縺ｯ `modules::offline::{OfflineManager, models::*}` 縺ｫ逶ｴ謗･萓晏ｭ倥＠縺ｦ縺翫ｊ縲、pplication 螻､蜀・〒 SQLx 讒矩菴難ｼ・OfflineAction`, `CacheMetadata` 縺ｪ縺ｩ・峨ｒ縺昴・縺ｾ縺ｾ蜈ｬ髢九＠縺ｦ縺・ｋ縲・- `phase5_dependency_inventory_template.md` 縺ｧ `OfflineService` / `OfflineManager` 繧・High 髮｣譏灘ｺｦ鬆・岼縺ｨ縺励※迚ｹ螳壽ｸ医∩縲・nfrastructure 縺ｸ縺ｮ遘ｻ陦後→謚ｽ雎｡蛹悶′豎ゅａ繧峨ｌ縺ｦ縺・ｋ縲・- Phase 4 縺ｧ霑ｽ蜉縺輔ｌ縺滓ｩ溯・・・yncQueue縲＾ptimisticUpdate 縺ｪ縺ｩ・峨′ Manager 縺ｫ髮・ｸｭ縺励∬ｲｬ蜍吶′閧･螟ｧ蛹悶＠縺ｦ縺・ｋ縲・
## 迴ｾ迥ｶ縺ｮ隱ｲ鬘・1. Application 螻､縺ｮ蜈ｬ髢句梛縺・Legacy 繝｢繧ｸ繝･繝ｼ繝ｫ萓晏ｭ・(`modules::offline::models::*`) 縺ｫ縺ｪ縺｣縺ｦ縺・ｋ縲・2. `serde_json::Value` 繧偵し繝ｼ繝薙せ蜀・〒螟夂畑縺励√ラ繝｡繧､繝ｳ繝｢繝・Ν縺御ｸ肴・迸ｭ縲・3. OfflineManager 縺ｸ縺ｮ蜻ｼ縺ｳ蜃ｺ縺励′蜷梧悄繝ｭ繧ｸ繝・け/繧ｭ繝｣繝・す繝･繝ｭ繧ｸ繝・け/繧ｭ繝･繝ｼ謫堺ｽ懊〒豺ｷ蝨ｨ縺励√ユ繧ｹ繝亥ｮｹ譏捺ｧ縺御ｽ弱＞縲・4. SQLx 讒矩菴薙・螟画峩縺・Application 螻､縺ｾ縺ｧ豕｢蜿翫☆繧九◆繧√￣hase 5 縺ｮ繝ｬ繧､繝､蜀肴ｧ区・縺ｫ荳榊髄縺阪・
## 謠先｡医い繝ｼ繧ｭ繝・け繝√Ε

### 1. 繝昴・繝茨ｼ域歓雎｡蛹厄ｼ芽ｨｭ險・- 譁ｰ隕剰ｿｽ蜉: `kukuri-tauri/src-tauri/src/application/ports/offline_store.rs`
- 蠖ｹ蜑ｲ: 繧ｪ繝輔Λ繧､繝ｳ髢｢騾｣縺ｮ豌ｸ邯壼喧謫堺ｽ懊ｒ謚ｽ雎｡蛹悶＠縲√し繝ｼ繝薙せ縺ｯ繝昴・繝医・縺ｿ縺ｫ萓晏ｭ倥・- 謚ｽ雎｡蛹悶ｒ4縺､縺ｫ蛻・牡縺苓ｲｬ蜍吶ｒ譏守｢ｺ蛹悶☆繧九・  ```rust
  #[async_trait]
  pub trait OfflineActionStore {
      async fn save_action(&self, payload: OfflineActionDraft) -> Result<OfflineActionRecord, AppError>;
      async fn list_actions(&self, filter: OfflineActionFilter) -> Result<Vec<OfflineActionRecord>, AppError>;
      async fn mark_synced(&self, action_id: OfflineActionId, remote_id: Option<String>) -> Result<(), AppError>;
  }

  #[async_trait]
  pub trait SyncQueueStore {
      async fn enqueue(&self, item: SyncQueueItemDraft) -> Result<SyncQueueId, AppError>;
      async fn ensure_enqueued(&self, action: &OfflineActionRecord) -> Result<bool, AppError>;
      async fn pending_items(&self) -> Result<Vec<SyncQueueItem>, AppError>;
  }

  #[async_trait]
  pub trait CacheMetadataStore {
      async fn upsert_metadata(&self, update: CacheMetadataUpdate) -> Result<(), AppError>;
      async fn list_stale(&self) -> Result<Vec<CacheMetadataRecord>, AppError>;
      async fn cleanup_expired(&self) -> Result<u32, AppError>;
  }

  #[async_trait]
  pub trait OptimisticUpdateStore {
      async fn save(&self, update: OptimisticUpdateDraft) -> Result<OptimisticUpdateId, AppError>;
      async fn confirm(&self, id: OptimisticUpdateId) -> Result<(), AppError>;
      async fn rollback(&self, id: OptimisticUpdateId) -> Result<Option<String>, AppError>;
      async fn unresolved(&self) -> Result<Vec<OptimisticUpdateRecord>, AppError>;
  }
  ```
- Application 螻､縺ｧ縺ｯ縺薙ｌ繧峨・繝昴・繝医ｒ譚溘・縺・`OfflinePersistence` 繧呈ｳｨ蜈･縺励～OfflineService` 縺ｯ蛟､繧ｪ繝悶ず繧ｧ繧ｯ繝医→繝峨Γ繧､繝ｳ繝ｭ繧ｸ繝・け縺ｫ髮・ｸｭ縺吶ｋ縲・
### 2. 繝峨Γ繧､繝ｳ蛟､繧ｪ繝悶ず繧ｧ繧ｯ繝医・謨ｴ蛯・- 譌｢蟄倥・ `OfflineActionRecord`, `SyncResult`, `CacheStatusData` 縺ｪ縺ｩ繧・`domain::entities::offline`・域眠險ｭ・峨∈遘ｻ縺励ヾQLx 逕ｱ譚･縺ｮ `i64` / `String` 繧貞梛莉倥￠縺吶ｋ縲・  - 萓・ `OfflineActionId(String)`, `CacheKey(String)`, `SyncStatus(enum)`縲・- `serde_json::Value` 繧堤峩謗･霑斐＆縺壹～OfflinePayload`・亥・驛ｨ縺ｧ `serde_json::Value` 繧剃ｿ晄戟縺吶ｋ newtype・峨〒繝ｩ繝・・縺励ヰ繝ｪ繝・・繧ｷ繝ｧ繝ｳ繝昴う繝ｳ繝医ｒ譏守､ｺ縺吶ｋ縲・
### 3. Infrastructure 螳溯｣・・谿ｵ髫守ｧｻ陦・- 譁ｰ繝・ぅ繝ｬ繧ｯ繝医Μ: `kukuri-tauri/src-tauri/src/infrastructure/offline/`
  - `sqlite_store.rs`: SQLx 繝吶・繧ｹ縺ｮ螳溯｣・よ里蟄倥・ `OfflineManager` 繝ｭ繧ｸ繝・け繧貞・蜑ｲ遘ｻ讀阪・  - `mappers.rs`: SQLx Row 竊・繝峨Γ繧､繝ｳ蛟､繧ｪ繝悶ず繧ｧ繧ｯ繝医・螟画鋤縲・  - `mod.rs`: `OfflinePersistenceImpl` 繧貞・髢九・- 譌｢蟄倥・ `modules/offline` 縺ｯ Legacy 蛹悶＠縲∵ｮｵ髫守噪縺ｫ蜑企勁縲・
### 4. 繧ｵ繝ｼ繝薙せ螻､繝ｪ繝輔ぃ繧ｯ繧ｿ繝ｪ繝ｳ繧ｰ譁ｹ驥・- `OfflineService` 縺ｧ縺ｯ莉･荳九ｒ蠕ｹ蠎・
  - 繝昴・繝育ｵ檎罰縺ｧ繝・・繧ｿ蜿門ｾ励＠縲～SyncResult` 縺ｪ縺ｩ縺ｮ髮・ｨ医・縺ｿ諡・ｽ薙・  - UI 縺九ｉ蜿励￠蜿悶ｋ `serde_json::Value` 縺ｯ譁ｰ險ｭ mapper 縺ｧ繝舌Μ繝・・繧ｷ繝ｧ繝ｳ蠕後↓ `OfflinePayload` 縺ｸ螟画鋤縲・  - `manager` 繝輔ぅ繝ｼ繝ｫ繝峨ｒ `Arc<dyn OfflinePersistence>` 縺ｫ鄂ｮ謠帙・
## 谿ｵ髫守噪遘ｻ陦瑚ｨ育判

### Stage 0・域ｺ門ｙ: 1譌･・・1. 繝峨Γ繧､繝ｳ蛟､繧ｪ繝悶ず繧ｧ繧ｯ繝医→繝峨Λ繝輔ヨ/繝輔ぅ繝ｫ繧ｿ讒矩菴薙ｒ霑ｽ蜉・・domain::entities::offline`・峨・2. `phase5_dependency_inventory_template.md` 縺ｧ `OfflineService` 縺ｮ隱ｲ鬘碁・岼縺ｫ繝ｪ繝ｳ繧ｯ縲・
#### Stage 0 繧ｿ繧ｹ繧ｯ荳隕ｧ・・025蟷ｴ10譛・3譌･霑ｽ蜉・・| ID | 菴懈･ｭ蜀・ｮｹ | 蟇ｾ雎｡繝代せ/繝｢繧ｸ繝･繝ｼ繝ｫ | 繝√ぉ繝・け繝昴う繝ｳ繝・|
| --- | --- | --- | --- |
| OFF-S0-01 | 繧ｪ繝輔Λ繧､繝ｳ髢｢騾｣縺ｮ蛟､繧ｪ繝悶ず繧ｧ繧ｯ繝茨ｼ上お繝ｳ繝・ぅ繝・ぅ・・OfflineActionId`, `OfflinePayload`, `SyncStatus` 縺ｪ縺ｩ・峨ｒ Domain 螻､縺ｸ霑ｽ蜉縲・| `kukuri-tauri/src-tauri/src/domain/entities/offline/*`<br>`kukuri-tauri/src-tauri/src/domain/value_objects/offline/*` | 譌｢蟄・Application 蝙九→縺ｮ莠呈鋤諤ｧ繧堤｢ｺ隱阪＠縲ヾerde 豢ｾ逕溘ｒ莉倅ｸ弱・|
| OFF-S0-02 | 譁ｰ VO 縺ｸ縺ｮ螟画鋤繝倥Ν繝代ｒ Application 螻､縺ｫ莉ｮ螳溯｣・＠縲～OfflineService` 縺九ｉ縺ｮ蛻ｩ逕ｨ邂・園繧呈ｴ励＞蜃ｺ縺吶・| `kukuri-tauri/src-tauri/src/application/services/offline_service.rs` | 螟画鋤邂・園縺ｮ TODO 繧偵さ繝｡繝ｳ繝医〒谿九＠縲∝ｾ檎ｶ・Stage 1 縺ｧ蟾ｮ縺玲崛縺亥庄閭ｽ縺ｪ迥ｶ諷九↓縺吶ｋ縲・|
| OFF-S0-03 | `.sqlx` 繝・ぅ繝ｬ繧ｯ繝医Μ縺ｮ譌｢蟄倥ヵ繧｡繧､繝ｫ縺ｨ OfflineManager 縺ｮ SQL 繧呈｣壼査縺励＠縲∵ｺ門ｙ谿ｵ髫弱〒縺ｮ蜀咲函謌占ｦ∝凄繧貞愛螳壹・| `kukuri-tauri/src-tauri/.sqlx/`<br>`kukuri-tauri/src-tauri/src/modules/offline/manager.rs` | 蜍慕噪 SQL 縺ｮ縺溘ａ蜊ｳ譎ょ・逕滓・荳崎ｦ√□縺後ヾtage 2 縺ｧ `query!` 蟆主・譎ゅ↓ `cargo sqlx prepare` 縺悟ｿ・ｦ√↓縺ｪ繧狗せ繧偵Γ繝｢縲・|

#### OFF-S0-01 螳溯｣・Γ繝｢・・025蟷ｴ10譛・4譌･・・- `domain::value_objects::offline` 縺ｫ `OfflineActionId`繝ｻ`OfflinePayload`繝ｻ`SyncStatus`繝ｻ`SyncQueueStatus`繝ｻ`CacheKey` 縺ｪ縺ｩ縺ｮ蛟､繧ｪ繝悶ず繧ｧ繧ｯ繝医ｒ霑ｽ蜉縺励∝ｾ捺擂縺ｮ `String`・汁i64` 繝輔ぅ繝ｼ繝ｫ繝峨ｒ蝙倶ｻ倥￠縺励◆縲・- `domain::entities::offline` 縺ｧ縺ｯ `OfflineActionRecord`繝ｻ`SyncQueueItem`繝ｻ`CacheMetadataRecord`繝ｻ`OptimisticUpdateRecord`繝ｻ`SyncStatusRecord`繝ｻ`CacheStatusSnapshot` 縺ｪ縺ｩ繧貞ｮ夂ｾｩ縲Ａchrono::DateTime<Utc>` 縺ｧ繧ｿ繧､繝繧ｹ繧ｿ繝ｳ繝励ｒ謇ｱ縺・～serde_json::Value` 縺ｯ `OfflinePayload` 邨檎罰縺ｧ蛹・ｓ縺縲・- 譁ｰ隕上せ繧ｭ繝ｼ繝樔ｻ倥″縺ｧ `cargo check` 繧貞ｮ溯｡後＠縲√ン繝ｫ繝牙庄閭ｽ縺ｪ迥ｶ諷九ｒ遒ｺ隱阪４tage 1 莉･髯阪・縺薙ｌ繧峨・蝙九ｒ `OfflineService` 縺九ｉ蜿ら・縺吶ｋ繧医≧螟画鋤繝倥Ν繝代ｒ螳溯｣・ｺ亥ｮ壹・
### Stage 1・・dapter 蟆主・: 3譌･・・1. `application::ports::offline_store` 繧定ｿｽ蜉縺励～OfflineService` 縺ｫ豕ｨ蜈･繝昴う繝ｳ繝医ｒ逕ｨ諢擾ｼ域里蟄・`OfflineManager` 繧偵Λ繝・・縺吶ｋ證ｫ螳・Adapter 繧貞ｮ溯｣・ｼ峨・2. 證ｫ螳・Adapter (`LegacyOfflineManagerAdapter`) 繧・`modules/offline` 荳翫↓螳溯｣・＠縲√ユ繧ｹ繝医ｒ繝｢繝・け邨檎罰縺ｫ譖ｴ譁ｰ縲・3. `OfflineService` 蜀・・蝙句､画鋤繧偵ラ繝｡繧､繝ｳ蛟､繧ｪ繝悶ず繧ｧ繧ｯ繝医∈鄂ｮ謠幢ｼ井ｸ驛ｨ譌ｧ讒矩菴薙ｒ newtype 縺ｧ蛹・・・峨・
### Stage 2・・nfrastructure 遘ｻ陦・ 4譌･・・1. `infrastructure/offline/sqlite_store.rs` 繧剃ｽ懈・縺励～OfflineManager` 縺ｮ CRUD 繝ｭ繧ｸ繝・け繧堤ｧｻ讀阪・2. `LegacyOfflineManagerAdapter` 縺九ｉ譁ｰ繧､繝ｳ繝輔Λ螳溯｣・∈蟾ｮ縺玲崛縺医Ａmodules/offline` 縺ｯ read-only 繝ｩ繝・ヱ縺ｫ邵ｮ騾縲・3. SQLx 繝・せ繝茨ｼ・offline_service` 縺ｮ邨仙粋繝・せ繝茨ｼ峨ｒ `tests/integration/offline/*` 縺ｫ遘ｻ縺励．ocker 繧ｹ繧ｯ繝ｪ繝励ヨ繧呈峩譁ｰ縲・
#### Stage 2 螳溯｣・Γ繝｢・・025蟷ｴ10譛・4譌･・・- `SqliteOfflinePersistence` 繧貞ｮ溯｣・＠縲～modules::offline::manager` 縺九ｉ荳ｻ隕・CRUD 繝ｭ繧ｸ繝・け・医が繝輔Λ繧､繝ｳ繧｢繧ｯ繧ｷ繝ｧ繝ｳ菫晏ｭ倥・蜷梧悄縲√く繝｣繝・す繝･繝｡繧ｿ繝・・繧ｿ譖ｴ譁ｰ縲∝酔譛溘く繝･繝ｼ謫堺ｽ懊∵･ｽ隕ｳ逧・峩譁ｰ邂｡逅・∝酔譛溘せ繝・・繧ｿ繧ｹ譖ｴ譁ｰ・峨ｒ遘ｻ讀阪＠縺溘Ａuuid` / `chrono` / `sqlx::QueryBuilder` 繧呈ｴｻ逕ｨ縺励▽縺､縲∵綾繧雁､繧・`AppError` 繝吶・繧ｹ縺ｫ謨ｴ蛯吶・- `state.rs` 縺ｮ DI 繧呈眠螳溯｣・↓蛻・ｊ譖ｿ縺医～Arc<dyn OfflinePersistence>` 縺ｸ SQLite 繝励・繝ｫ繧堤峩謗･豕ｨ蜈･縲ゅ・繝ｬ繧ｼ繝ｳ繝・・繧ｷ繝ｧ繝ｳ螻､縺ｮ繝上Φ繝峨Λ繝ｼ繧・里蟄倥し繝ｼ繝薙せ縺ｯ繧､繝ｳ繧ｿ繝ｼ繝輔ぉ繧､繧ｹ縺ｮ縺ｿ蜿ら・縺吶ｋ讒区・縺ｫ邨ｱ荳縲・- OfflineService 縺ｮ繝ｦ繝九ャ繝医ユ繧ｹ繝医ｒ `LegacyOfflineManagerAdapter` 萓晏ｭ倥°繧・`SqliteOfflinePersistence` 縺ｸ蟾ｮ縺玲崛縺医√う繝ｳ繝｡繝｢繝ｪ SQLite 繧ｹ繧ｭ繝ｼ繝槫・譛溷喧蠕後↓蜷檎ｭ峨・繧ｷ繝翫Μ繧ｪ・井ｿ晏ｭ倥・蜷梧悄繝ｻ繧ｭ繝･繝ｼ繝ｻ繧ｭ繝｣繝・す繝･繝ｻ讌ｽ隕ｳ逧・峩譁ｰ・峨ｒ讀懆ｨｼ縲・- `infrastructure/offline/mod.rs` 縺九ｉ Legacy 繧｢繝繝励ち縺ｮ蜀阪お繧ｯ繧ｹ繝昴・繝医ｒ螟悶＠縲∵眠 API 繧貞━蜈井ｽｿ逕ｨ縺ｧ縺阪ｋ繧医≧蜈ｬ髢矩擇繧呈紛逅・４tage 3 縺ｧ縺ｮ Legacy 邵ｮ騾繧貞ｮｹ譏薙↓縺吶ｋ縺溘ａ縲∵立繧｢繝繝励ち縺ｯ繝｢繧ｸ繝･繝ｼ繝ｫ逶ｴ荳九〒邯ｭ謖√・- 繝薙Ν繝画紛蜷域ｧ遒ｺ隱阪→縺励※ `cargo fmt` 竊・`cargo clippy -- -D warnings` 竊・`./scripts/test-docker.ps1 rust` 繧貞ｮ溯｡後・ocker Rust 繝・せ繝亥ｮ瑚ｵｰ縺ｧ莠呈鋤諤ｧ繧堤｢ｺ隱搾ｼ域里遏･縺ｮ `Nip10Case::description` 隴ｦ蜻翫・縺ｿ・峨・
### Stage 3・・egacy 隗｣菴・ 2譌･・・1. `modules/offline` 繧貞炎髯､縺励～infrastructure/offline::{rows,mappers,sqlite_store,reindex_job}` 縺ｫ邨ｱ蜷医０fflineReindexJob 縺ｯ譁ｰ Persistence 繧堤峩謗･蛻ｩ逕ｨ縺吶ｋ縲・2. `state.rs` 縺ｮ DI 縺ｨ繝・せ繝郁ｳ・肇繧貞姐譁ｰ縺励～SqliteOfflinePersistence` / `OfflineReindexJob` 繧貞・譛峨・3. Documentation 譖ｴ譁ｰ・域悽繝峨く繝･繝｡繝ｳ繝医～phase5_dependency_inventory_template.md` 縺ｪ縺ｩ・峨→繝・せ繝医・陬懷ｮ後・
#### Stage 3 螳溯｣・Γ繝｢・・025蟷ｴ10譛・5譌･・・- `modules/offline` 荳蠑擾ｼ・anager/models/reindex/tests・峨ｒ蜑企勁縺励∬｡梧ｧ矩縺ｯ `infrastructure/offline/rows.rs` 縺ｨ mapper 縺ｸ遘ｻ讀阪・egacy adapter 繧よ彫蜴ｻ縲・- `SqliteOfflinePersistence` 縺ｫ蜷梧悄繧ｭ繝･繝ｼ繝ｻ繧ｭ繝｣繝・す繝･繝ｻ讌ｽ隕ｳ逧・峩譁ｰ繝ｻ蜷梧悄迥ｶ諷九・蜿門ｾ・API 繧定ｿｽ蜉縺励～OfflineReindexJob` 繧呈眠險ｭ繝｢繧ｸ繝･繝ｼ繝ｫ縺ｧ蜀榊ｮ溯｣・ＡAppState` 縺九ｉ縺ｯ `Arc<SqliteOfflinePersistence>` 繧貞・譛峨＠縺ｦ繧ｸ繝ｧ繝悶ｒ逕滓・縲・- `state.rs` 縺九ｉ Legacy `OfflineManager` 萓晏ｭ倥ｒ髯､蜴ｻ縺励．I 繧・`OfflineReindexJob` + `OfflineService` 縺ｮ莠檎ｵ瑚ｷｯ縺ｫ謨ｴ逅・・- Rust 繝ｦ繝九ャ繝医ユ繧ｹ繝医ｒ `sqlite_store.rs` / `reindex_job.rs` 蜀・∈蜀埼・鄂ｮ縺励※繧ｫ繝舌Ξ繝・ず繧堤ｶｭ謖√Ａcargo test` 縺ｯ繝ｭ繝ｼ繧ｫ繝ｫ繝ｪ繝ｳ繧ｯ繧ｨ繝ｩ繝ｼ縺ｧ螟ｱ謨暦ｼ・cc` 邨檎罰縺ｧ iroh 萓晏ｭ倥Λ繧､繝悶Λ繝ｪ link 荳榊庄・峨□縺後√ユ繧ｹ繝医さ繝ｼ繝芽・菴薙・繝薙Ν繝峨∪縺ｧ遒ｺ隱肴ｸ医∩縲・
### `.sqlx` 蠖ｱ髻ｿ繝｡繝｢・・025蟷ｴ10譛・3譌･隱ｿ譟ｻ髢句ｧ具ｼ・- 迴ｾ陦・`OfflineManager` 縺ｯ `sqlx::query` / `query_as` 繧貞虚逧・SQL 譁・ｭ怜・縺ｧ蜻ｼ縺ｳ蜃ｺ縺励※縺翫ｊ縲～.sqlx/` 縺ｫ繝励Μ繧ｳ繝ｳ繝代う繝ｫ貂医∩繧ｯ繧ｨ繝ｪ縺ｯ蟄伜惠縺励↑縺・・- Stage 2 縺ｧ Repository 繧・Infrastructure 螻､縺ｸ遘ｻ陦後＠縲～query!` / `query_as!` 繧呈治逕ｨ縺吶ｋ蝣ｴ蜷医・ `cargo sqlx prepare` 繧貞・螳溯｡後＠縺ｦ `.sqlx/query-*.json` 繧堤函謌舌☆繧句ｿ・ｦ√′縺ゅｋ縲・- `.sqlx` 蜀咲函謌先凾縺ｯ `scripts` 驟堺ｸ九・ CI 謇矩・ｼ・./scripts/test-docker.ps1 rust`・峨→謨ｴ蜷医ｒ遒ｺ隱阪＠縲√い繝ｼ繝・ぅ繝輔ぃ繧ｯ繝医ｒ繝ｪ繝昴ず繝医Μ縺ｫ蜷ｫ繧√ｋ縺薙→縲・
## 繝・せ繝域婿驥・- Stage 1: 譌｢蟄倥Θ繝九ャ繝医ユ繧ｹ繝医ｒ繝｢繝・け蟾ｮ縺玲崛縺医〒騾夐℃縺輔○繧九・- Stage 2: 譁ｰ險ｭ縺ｮ integration 繝・せ繝茨ｼ・QLite 螳・DB・峨ｒ Docker 繧ｸ繝ｧ繝悶↓霑ｽ蜉縲・- Stage 3: Regression 逕ｨ縺ｫ `scripts/test-docker.ps1 offline` 繧定ｿｽ蜉縺励，I 縺ｫ逋ｻ骭ｲ縲・
## 繧ｪ繝ｼ繝励Φ隱ｲ鬘・- 繧ｪ繝輔Λ繧､繝ｳ繧ｭ繝･繝ｼ縺ｮ蜀埼√・繝ｪ繧ｷ繝ｼ繧偵←縺薙〒邂｡逅・☆繧九°・・ervice 縺・Infrastructure・峨４tage 2 縺ｧ豎ｺ螳壹・- `serde_json::Value` 縺ｮ繝舌Μ繝・・繧ｷ繝ｧ繝ｳ繧ｹ繧ｭ繝ｼ繝槫喧・・SON Schema or custom validator・峨↓縺､縺・※縺ｯ蛻･繧ｿ繧ｹ繧ｯ蛹悶・- 繝槭う繧ｰ繝ｬ繝ｼ繧ｷ繝ｧ繝ｳ谿ｵ髫弱〒 `.sqlx/` 縺ｮ蜀咲函謌舌′蠢・ｦ√↓縺ｪ繧九◆繧√ヾtage 2 螳御ｺ・ｾ後↓蟇ｾ蠢懊・

## 進捗ログ

- 2025年10月25日: Stage2-3 で OfflineService の統合テストを `application/services/offline_service.rs` から `tests/integration/offline/mod.rs` へ移設。SQLite スキーマ初期化と永続化の DI を再確認し、Docker/CI で `cargo test --test offline_integration` を実行できるように整備。


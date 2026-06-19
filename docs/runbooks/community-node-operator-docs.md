# Community Node Operator Docs Generator (`cn-operator`)

最終更新日: 2026-06-19

## 目的

community node 運営者が `operator-config.yaml` を単一の入力元として、有効化した機能に対応した
運営者向け文書群（利用規約 / プライバシーポリシー / 外部送信表示 / 電気通信届出補助資料 /
ネットワーク構成説明 / server manifest）を**決定論的に**生成できるようにする。

これは #352 の実装であり、`docs/architecture/p2p-first-community-node-responsibility-boundary.md`
の責任境界を共通前提とする。community node を中央 SNS 運営者にするためのものではなく、
P2P network の補助層を個人・小規模グループでも説明可能に運用できるようにするためのもの。

## Phase A / Phase B（宣言と実行可能の分離）

`cn-operator` は各機能を capability として扱い、`availability` を持たせている。

- **Phase A (`Available`)**: 現行 community node 実装（auth / consent / bootstrap / topic
  rendezvous / iroh relay）またはデプロイ構成（Cloudflare / analytics / crash report /
  blob cache / private message storage / push）として提供できる capability。
  生成文書では「運用中」として開示してよい。
- **Phase B (`Planned`)**: 現行実装に存在しない capability（`community_index` / `moderation` /
  `community_local_trust` / `report_endpoint`）。config で宣言できるが、生成文書では
  「計画中（この配布物では未提供）」として扱い、運用中の外部送信・データ取扱い開示には載せない。

Phase B capability を有効化するには、config に `acknowledge_planned_capabilities: true` を
明示する必要がある。これがないと検証で失敗する。実体のない「運用中」開示を生成しないためのガード。

## サブコマンド

```bash
# サンプル config を出力
cn-operator init --out operator-config.yaml

# config を検証（Phase B 承認ガードを含む）
cn-operator validate-config --config operator-config.yaml

# 文書群を生成
cn-operator generate-docs --config operator-config.yaml --out-dir dist/operator-docs

# 生成済み文書と config の drift を検出（差分があれば non-zero exit）
cn-operator check-disclosures --config operator-config.yaml --out-dir dist/operator-docs
```

cargo から直接実行する場合:

```bash
cargo run -p kukuri-cn-operator --bin cn-operator -- generate-docs \
  --config operator-config.yaml --out-dir dist/operator-docs
```

## profile

`profile` は features の既定値を与え、個別の `features` キーで上書きできる。

- `minimal`: index / moderation / community-local trust（いずれも計画中）。relay / cache /
  analytics / crash report は無効。
- `relay-enabled`: minimal + 専用 iroh relay + 暗号化済み traffic fallback 開示。
- `full-service`: relay-enabled + blob cache + push 通知 + report endpoint。analytics /
  crash report は任意（既定無効）。

## 生成される文書

```text
dist/operator-docs/
  server-manifest.json          # #355: 型付き manifest schema（下記参照）
  network-diagram.md
  telecom-notification-draft.md
  service-description-draft.md
  terms.md
  privacy-policy.md
  external-transmission-notice.md
  abuse-policy.md
  moderation-policy.md
  data-retention-policy.md
  prior-consultation-email.md
```

各文書には「法的助言ではない」旨の注記が含まれる。最終判断は運営者自身および総合通信局・
専門家への確認が必要。

## server-manifest.json（#355）

`server-manifest.json` は型付きの共有スキーマ（`kukuri_cn_operator::CommunityNodeManifest`）として
定義される。public manifest endpoint (#356) や client（dependency 表示 / report routing /
consent UI）が同じ型を共有して扱えるようにするためのもの。主な構造:

- `node_role`: `default-onboarding-node` / `community-node` / `relay-assist` / `index-node` /
  `moderation-node` / `trust-signal-node`。未指定なら有効 capability から推定（既定 `community-node`）。
  default onboarding node は明示することで third-party community node と区別できる。
- `capabilities`: 全 capability の有効・無効。
- `capability_scope`: `available_enabled`（Phase A）/ `planned_enabled`（Phase B）を分離。
- `authority_scope`: `applies_to`（有効 capability から導出 + operator が `additional_applies_to`
  で拡張可能）/ `does_not_apply_to`（安全な default。operator が上書き可能）。
- `p2p_boundary`: identity / profile / social graph / content truth source / network-wide
  authority をすべて `false` 宣言。これは kukuri の P2P-first 設計の不変条件であり、operator は
  変更できない（community node を home server / central operator と誤解させないため）。

config からの設定例:

```yaml
manifest:
  node_role: default-onboarding-node
  authority_scope:
    additional_applies_to:
      - custom_scope
    does_not_apply_to: null   # 未指定なら安全な default
```

## 決定論性と CI

出力は wall-clock に依存せず、version は config 由来（`manifest.manifest_version`）。
同じ config からは同じ出力が得られる。CI では `check-disclosures` で生成済み文書と config の
drift を検出できる。

## 関連

- #352（本実装）, #355（manifest authority scope）, #356（public manifest endpoint）,
  #359（capability 別リスクガイド）
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`

# Community Node Production Rollout / Live Verification

最終更新日: 2026-08-07

## 目的

この runbook は、GCP `low-cost` Community Node の production image 更新から Terraform、
startup、readiness、実クライアント投稿の追跡、公開境界の復旧までを一続きで実行するための
日常運用手順である。Terraform の初期構築と各変数の説明は
`docs/runbooks/community-node-gcp-terraform.md`、クライアント配布は
`docs/runbooks/release.md` を参照する。

この手順で特に防ぐ事故は次の4つ。

- workflow log に出た smoke build の digest を production digest と誤認する
- `metadata_startup_script` の差分だけで既存 VM を意図せず置換する
- image pull 中に Docker 領域が枯渇し、更新途中で startup が止まる
- health/readiness だけで完了とし、実投稿の media fetch・allow-only index・非残留を見落とす

## 0. 変数と記録先

作業開始前に値を固定し、同じ値を作業記録へ残す。secret 値は記録しない。

```bash
export KUKURI_GCP_PROJECT="YOUR_PROJECT"
export KUKURI_GCP_ZONE="asia-northeast1-a"
export KUKURI_VM="kukuri-cn-vm"
export KUKURI_REPO="KingYoSun/kukuri"
export KUKURI_MAIN_SHA="<40-character-main-sha>"
export KUKURI_SHA_TAG="sha-$(printf '%s' "$KUKURI_MAIN_SHA" | cut -c1-12)"
```

PowerShell:

```powershell
$env:KUKURI_GCP_PROJECT = 'YOUR_PROJECT'
$env:KUKURI_GCP_ZONE = 'asia-northeast1-a'
$env:KUKURI_VM = 'kukuri-cn-vm'
$env:KUKURI_REPO = 'KingYoSun/kukuri'
$env:KUKURI_MAIN_SHA = '<40-character-main-sha>'
$env:KUKURI_SHA_TAG = 'sha-' + $env:KUKURI_MAIN_SHA.Substring(0, 12)
```

記録するもの:

- main SHA、PR、Fast CI run、Community Node Images run
- 4 image の **registryで解決したdigest**
- apply前backup object、generation、size
- Terraform plan の add/change/destroy と replacement の有無
- startup script の desired / metadata SHA-256
- container、timer、readiness、truth/projection、scan/media metrics
- 障害・復旧操作、削除したもの、rollback先digest

## 1. publish と digest の確定

1. PRの全check成功を確認してから対象変更をmainへ反映する。
2. main の `Kukuri Fast` と `Kukuri Community Node Images` を最後まで確認する。
3. image job 成功後、GHCR に存在する `sha-<12桁>` tag から digest を取得する。
4. digest参照が実際にmanifestとして解決できることを検査する。

```bash
gh pr checks <pr-number> --repo "$KUKURI_REPO" --watch
gh pr merge <pr-number> --repo "$KUKURI_REPO" --squash --delete-branch
```

repositoryにrequired checkが設定されていない場合、`gh pr merge --auto` はcheck待ちにならず即時merge
されることがある。CI成功後にmergeする運用ではauto-merge登録に依存せず、`gh pr checks --watch` の
成功を明示確認してからmergeする。merge後に起動するmain workflowも別に完走確認する。

```bash
for image in kukuri-cn-user-api kukuri-cn-iroh-relay kukuri-cn-cli kukuri-cn-indexer; do
  ref="ghcr.io/kingyosun/${image}:${KUKURI_SHA_TAG}"
  digest="$(docker buildx imagetools inspect "$ref" --format '{{println .Manifest.Digest}}')"
  test -n "$digest"
  docker manifest inspect "ghcr.io/kingyosun/${image}@${digest}" >/dev/null
  printf '%s@%s\n' "ghcr.io/kingyosun/${image}" "$digest"
done
```

workflow log の最初の `sha256:` をコピーしてはならない。`cn-indexer` job は publish対象とは別に
positive smoke用imageを先にbuildすることがあり、そのdigestはGHCRにpushされない。真実源は
job成功後のregistry manifestである。VMからも `docker manifest inspect <image>@<digest>` が通ることを
確認すれば、公開範囲・認証・digestの取り違えをapply前に検出できる。

取得した4参照を `operator-config.yaml` と
`infra/terraform/envs/low-cost/terraform.tfvars` の両方へ反映し、差異が無いことを確認する。

## 2. apply前 gate

### 2.1 検証とbackup

```bash
cargo xtask cn-check
cargo xtask cn-test
cargo xtask cn-e2e

terraform -chdir=infra/terraform fmt -check -recursive
terraform -chdir=infra/terraform/envs/low-cost init
terraform -chdir=infra/terraform/envs/low-cost validate
terraform -chdir=infra/terraform/envs/low-cost plan -out=tfplan
```

本番DBのbackupを先に取得し、GCS objectのgenerationとsizeを記録する。既存の
`kukuri-backup.service` を使う場合:

```bash
gcloud compute ssh "$KUKURI_VM" \
  --project "$KUKURI_GCP_PROJECT" --zone "$KUKURI_GCP_ZONE" \
  --tunnel-through-iap --command 'sudo systemctl start kukuri-backup.service'
gcloud storage ls -l "gs://<backup-bucket>/postgres/**"
```

### 2.2 replacement gate

`terraform plan` に `module.vm.google_compute_instance.vm` のreplacementがあれば、そのままapplyしない。
次をすべて満たす場合だけ後述のstartup metadata in-place同期を使える。

- replacement理由が `metadata_startup_script` のみ
- machine type、boot disk、network、service account、attached PDに同時変更が無い
- Postgres / indexerのPDと最新backupを確認済み
- DNS / ACME / 証明書を保持する必要があり、計画停止でVM置換するよりin-placeが安全

他のreplace理由が混じる場合はmaintenance windowを取り、通常のVM置換として扱う。

## 3. startup metadata の in-place 同期（限定的なbreak-glass手順）

`metadata_startup_script` だけの置換を避ける場合、review済みplanからdesired scriptを抽出し、
Compute Engine metadataの `startup-script` だけを更新する。作業用fileはsecret値を含まない設計だが、
operator configや内部URLを含み得るので一時fileとして扱う。

`jq` を使う例:

```bash
rollout_tmp="$(mktemp -d)"
trap 'rm -f -- "$rollout_tmp/startup.sh"; rmdir -- "$rollout_tmp"' EXIT

terraform -chdir=infra/terraform/envs/low-cost show -json tfplan \
  | jq -j '.resource_changes[]
      | select(.address == "module.vm.google_compute_instance.vm")
      | .change.after.metadata_startup_script' \
  >"$rollout_tmp/startup.sh"
test -s "$rollout_tmp/startup.sh"
sha256sum "$rollout_tmp/startup.sh"

gcloud compute instances add-metadata "$KUKURI_VM" \
  --project "$KUKURI_GCP_PROJECT" --zone "$KUKURI_GCP_ZONE" \
  --metadata-from-file "startup-script=$rollout_tmp/startup.sh"
```

PowerShell 7でplanから抽出する場合は、BOMを付けずLFを維持する。

```powershell
$plan = terraform -chdir=infra/terraform/envs/low-cost show -json tfplan |
  ConvertFrom-Json -Depth 100
$script = $plan.resource_changes |
  Where-Object address -eq 'module.vm.google_compute_instance.vm' |
  ForEach-Object { $_.change.after.metadata_startup_script }
if ([string]::IsNullOrWhiteSpace($script)) { throw 'desired startup script not found' }
$tmp = Join-Path ([IO.Path]::GetTempPath()) 'kukuri-startup.sh'
[IO.File]::WriteAllText($tmp, $script, [Text.UTF8Encoding]::new($false))
Get-FileHash -Algorithm SHA256 $tmp
gcloud compute instances add-metadata $env:KUKURI_VM `
  --project $env:KUKURI_GCP_PROJECT --zone $env:KUKURI_GCP_ZONE `
  --metadata-from-file "startup-script=$tmp"
```

metadata server側のbytesと一致することを確認する。

```bash
gcloud compute ssh "$KUKURI_VM" \
  --project "$KUKURI_GCP_PROJECT" --zone "$KUKURI_GCP_ZONE" \
  --tunnel-through-iap --command \
  "curl -fsS -H 'Metadata-Flavor: Google' \
   http://metadata.google.internal/computeMetadata/v1/instance/attributes/startup-script \
   | sha256sum"
```

古い `tfplan` はreplacementを含むため絶対にapplyしない。metadata同期後にplanを作り直し、VMの
replacementが消えたことと、残るadd/change/destroyを再reviewしてから新しいplanだけをapplyする。

```bash
terraform -chdir=infra/terraform/envs/low-cost plan -out=tfplan-safe
terraform -chdir=infra/terraform/envs/low-cost apply tfplan-safe
terraform -chdir=infra/terraform/envs/low-cost plan   # 最終的に No changes
```

`add-metadata` は既存metadataを保持して指定keyを更新する。Compute APIの `setMetadata` を直接使う
場合は、必ず現在のfingerprintと全metadata itemを読み、他keyを欠落させない。

## 4. startup と容量枯渇の復旧

Container-Optimized OSではCompose v2 plugin discoveryを前提にしない。必ずTerraformが配置した
standalone binaryを使う。

```bash
cd /var/lib/kukuri/community-node
COMPOSE=/var/lib/toolbox/kukuri/bin/docker-compose
sudo "$COMPOSE" ps
```

startup前に容量を確認する。

```bash
df -h /var/lib/docker
sudo docker system df
sudo docker image ls --filter dangling=true
```

新旧4 imageを同時に保持できる空きが無ければ、先にmaintenance判断を行う。startupは次で再実行する。

```bash
sudo systemctl restart google-startup-scripts.service
sudo journalctl -u google-startup-scripts.service -f
```

成功条件はjournalの `[kukuri-startup] bootstrap complete` と、対象containerのrunning/healthy。

`no space left on device` で止まった場合、既存containerがhealthyなら慌ててvolumeを消さない。
次の順で復旧する。

1. `docker ps -a` で稼働・停止containerを記録する。
2. `docker system df` と `df -h /var/lib/docker` を記録する。
3. dangling imageだけを確認する。
4. `sudo docker image prune -f` を実行する。
5. 回収量と削除対象がregistryから再pull可能であることを記録する。
6. startup serviceを再実行する。

rollout中に `docker system prune`、`docker image prune -a`、`docker volume prune` は使わない。
特にPostgres / ArcadeDB / indexer dataのvolume・PDを容量回収対象にしてはならない。

## 5. post-deploy verification

### 5.1 image / container / timer

```bash
cd /var/lib/kukuri/community-node
COMPOSE=/var/lib/toolbox/kukuri/bin/docker-compose
sudo "$COMPOSE" ps
for container in community-node-cn-user-api-1 community-node-cn-iroh-relay-1 \
  community-node-cn-indexer-1; do
  sudo docker inspect "$container" \
    --format '{{.Name}}|{{.Config.Image}}|{{index .Config.Labels "org.opencontainers.image.revision"}}'
done

for unit in kukuri-readiness.timer kukuri-monitor.timer \
  kukuri-backup.timer kukuri-relation-analyze.timer; do
  sudo systemctl is-enabled "$unit"
  sudo systemctl is-active "$unit"
done
```

### 5.2 readiness

```bash
sudo systemctl start kukuri-readiness.service
sudo journalctl -u kukuri-readiness.service -n 100 --no-pager
```

最低条件:

- `ready=true fail=0 unknown=0`
- provider credentialが全slotでpass
- `permanent_blob_storage_disabled` がpass
- worker running、supported public scopes opened、sync / ingest fresh
- `scan_errors=0` かつ失敗からallowへのfallbackが0
- Postgres truthとArcadeDB projectionが一致
- relation analysis recent

readinessの成功だけではlive media確認の代わりにならない。

### 5.3 public surface

```bash
curl -fsS "https://<api-domain>/healthz"
curl -fsS "https://<relay-domain>/ping"
curl -fsS "https://<api-domain>/.well-known/kukuri/community-node.json"
curl -fsS "https://<api-domain>/v1/node/manifest"
```

manifestが示すterms / privacy / external-transmission / moderation-policy / abuse-policy /
data-retentionもHTTP 200と `text/markdown; charset=utf-8` を確認する。index / trust surfaceは、
有効時に未認証401または入力不備400となり、構成未完了を示す404へ戻っていないことを確認する。

### 5.4 monitoring通知とlog / secret監査

notification channelを追加・変更したrolloutでは、Terraformが全Community Node alert policyへ同じ
channelを付けたことをAPIで照合する。Email channelはCloud Monitoring側の `enabled=true` だけで
完了とせず、受信側でOPENED / CLOSEDの両方を確認する。

配送試験は本番policyを書き換えず、`[TEST]` prefixの一時policyを作る。既存custom metricへ短いtest値を
送り、一時policyでは即時評価、本番policyの継続条件（既定5分）より短く終了させる。OPENED受信後に
正常値を送り、CLOSED受信を確認して一時policyを削除する。最後に次を記録する。

- channel resource name、type、enabled
- 本番policy総数とchannel添付数
- test signal開始 / 復旧時刻
- OPENED / CLOSED受信
- 残存 `[TEST]` policy 0件、notification error 0件

logのsecret非含有監査では、secret値を `grep "$SECRET" ...` のようにcommand lineへ載せてはならない。
値は権限0700の一時directoryへ取得し、root-only scriptのprocess内で読み、出力はsecret IDごとの
match countだけにする。journal、startup log、全稼働container logを対象にし、`matches=0` を記録する。

誤ってsecret値をargvやjournalへ出した場合は、そこで監査を止める。対象secretをrotateし、該当する
journal archive / container logを保持方針に従ってrotate・vacuumした後、新旧両方の値で0件を再確認する。
秘密値そのものをincident記録へ転記しない。

## 6. 実クライアントのbenign media確認

実在の違法mediaや疑わしいmediaを検証に使わない。権利上問題のない小さな画像をpublic topicへ投稿し、
JST時刻、topic、post object ID、media hashを記録する。

### 6.1 index truth

VM上でobject IDを64桁hexに限定してから照会する。

```bash
OBJECT_ID="<64-hex-post-id>"
case "$OBJECT_ID" in (*[!0-9a-f]*|'') echo 'invalid object id' >&2; exit 1;; esac
test "${#OBJECT_ID}" -eq 64

PG_CONTAINER="$(sudo docker ps -qf name=cn-postgres)"
sudo docker exec "$PG_CONTAINER" sh -lc \
  "psql -U \"\$POSTGRES_USER\" -d \"\$POSTGRES_DB\" -At -F '|' \
   -c \"SELECT scope_kind, scope_id, object_id, author_pubkey, verdict_action, critical, indexed_at
       FROM cn_index.index_entries WHERE object_id = '$OBJECT_ID';\""
```

benign投稿の成功条件は対象rowが `public_topic`、期待topic、`allow`、`critical=false` であること。

### 6.2 ephemeral fetch / 非残留

```bash
MEDIA_HASH="<64-hex-media-hash>"
case "$MEDIA_HASH" in (*[!0-9a-f]*|'') echo 'invalid media hash' >&2; exit 1;; esac
test "${#MEDIA_HASH}" -eq 64

sudo docker logs --since 30m community-node-cn-indexer-1 2>&1 \
  | grep -F "$MEDIA_HASH"
```

次の組を確認する。

1. `fetch local miss, trying remote peers`
2. `ephemeral fetch remote transfer completed`
3. 別provider処理または後続scanでも同じhashが再び `fetch local miss` になる

3はremote bytesがlocal blob storeへ追加されていない実機証跡になる。あわせてreadinessの
`permanent_blob_storage_disabled` と `media_fetch(success/unavailable/timeout/oversize)` を記録する。
一度のtransfer successだけで恒久非残留と判定しない。

### 6.3 非表出時の切り分け

- `configured_peer_count=0` / active peerなし: docs participantのpeer情報がmedia BlobServiceへ
  伝播しているか、running imageのrevision、peer refresh logを確認する。
- remote transfer成功後にquarantine: providerのlabel / scoreとrouter結果を確認する。
  clean classifier結果は `Completed` かつlabel/scoreなしであり、critical capabilityを持つことだけを
  検知根拠にしてはならない。
- 過去のscanで保存されたblob verdict rowは、再deploy後の現在のpost surfacingを単独では表さない。
  DBを手動修正せず、対象postの現在のindex truth、最新worker metrics、対象hashの最新logを組み合わせる。
- provider unavailable / timeout / oversize / scan error時は非表出が正しい。allowへ手動fallbackしない。

## 7. 検証用DNSを残す場合の運用境界

届出・公開判断前に検証用DNSを残す場合は、不特定の新規参加を許可したまま完了しない。
DNSを閉じない場合はadmissionを `invite` へ切り替える。既存active subscriberは継続利用できる。

```bash
cd /var/lib/kukuri/community-node
CLI_IMAGE="ghcr.io/<owner>/kukuri-cn-cli@sha256:<verified-digest>"

sudo docker run --rm --network community-node_default --env-file .env \
  "$CLI_IMAGE" admission show
sudo docker run --rm --network community-node_default --env-file .env \
  "$CLI_IMAGE" admission set-mode --mode invite
sudo docker run --rm --network community-node_default --env-file .env \
  "$CLI_IMAGE" admission show
```

期待値は `admission mode: invite`。`.env` の値や `docker compose config` を作業logへ出力しない。

## 8. rollback判断

次のいずれかなら、新imageでの調査を続ける前にprevious digestへ戻す。

- migration後にAPI / indexerがhealthyへ戻らない
- required providerが継続的に失敗しreadinessが閉じたまま
- truth/projection不一致が再投影待ち時間を超えて続く
- benign contentが誤ってallow、またはunsafe contentが表出する安全性regression
- startup再実行後も容量・証明書・networkの障害が解消しない

rollbackでもtagではなく、直前に記録した4つのdigestを使う。apply前backupを保持し、DB schemaを
戻す必要がある変更では専用のmigration rollback手順が無い限りDBを上書きしない。復旧後に
readiness、public surface、実投稿を再検証する。

## 9. 完了記録テンプレート

```text
main / PR:
Fast CI / image workflow:
digests (user-api / relay / CLI / indexer):
backup object / generation / size:
Terraform plan/apply/final plan:
startup desired/server SHA-256:
container / timer:
readiness:
truth / projection / relation:
live post ID / media hash / posted_at:
media local-miss -> ephemeral-success -> later local-miss:
admission / DNS boundary:
incident / recovery / deleted resources / recoverability:
rollback digests:
```

## 関連

- `docs/runbooks/community-node-gcp-terraform.md`
- `docs/runbooks/community-node-operator-docs.md`
- `docs/runbooks/openai-compatible-vlm.md`
- `docs/runbooks/project-arachnid-shield.md`
- `docs/runbooks/release.md`

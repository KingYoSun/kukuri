# ADR 0048: 端末内データの暗号化バックアップと復元

## Status

Accepted

## Context

- Issue #855（Parent #853、Depends on #859）。#859 は本人性を表すアカウント鍵だけを移行できるが、SQLite、未同期データ、下書き、添付、設定、非公開チャネル能力は移行しない。
- #859 以後は複数アカウントを扱うため、初版の完全バックアップは「現在選択中の1アカウント」を単位とする。複数アカウントはそれぞれバックアップする。
- バックアップには秘密鍵、DM、非公開チャネル能力、招待情報、未公開下書きが含まれ得る。平文の一時アーカイブを作らず、利用者が指定したパスフレーズで暗号化する必要がある。

## Decision

### 1. 1ファイル、1アカウント

- `kukuri-device-backup.v1` は、1アカウントの管理情報と全entryをまとめた単一ファイルとする。
- ファイルは固定header、暗号化manifest、連番の暗号化chunkからなる。大容量Blobをfrontend IPCやメモリへ一括展開しない。
- 作成先と同じdirectoryの一時ファイルへ書き、完了時だけrenameする。失敗またはcancel時は一時ファイルを削除する。

### 2. 暗号化と検証

- 鍵導出は#859と同じArgon2id、暗号化はXChaCha20-Poly1305を使う。headerの形式版、KDF parameter、salt、nonce prefixを各chunkのAADへ束縛する。
- manifestは暗号化し、公開鍵、作成アプリ版、entry名、長さ、BLAKE3 hash、含有範囲を保持する。frontend state自体は暗号化entryに置く。
- chunk番号をnonceとAADへ含め、欠落、並べ替え、重複、切詰め、余分なchunkを拒否する。未知形式版、KDF上限超過、entry数・entry長・総容量の上限超過は復元前に拒否する。
- パスフレーズ、秘密鍵、復号内容はログ、診断、`Debug`、進捗eventへ出さない。

### 3. 対象と除外

- 必須: account key、整合したSQLite、discovery/Community Node接続設定、private channel能力、gossip購読状態、Community Node招待情報、iroh Docs/Blobの端末内状態（endpoint secretを除く）、下書き、workspace layout、theme、locale。
- 移行不可: iroh endpoint secret、Community Node bearer token、実行中session、通知cursor、OS通知権限。
- 再同意: app-level同意、Community Node同意、18歳以上の自己申告。これらの記録はバックアップへ含めない。成人向け表示設定も含めず、復元後はOFFとする。
- バックアップはCommunity Node、他端末、Direct P2P参加者が保持するcopyを削除または巻き戻さない。

### 4. 整合スナップショット

- active runtimeを停止してSQLite pool、Docs、Blob、background syncを閉じた後にファイルを列挙する。main DBを正とし、close timeoutなどで残ったWAL/shmも失わないよう存在時は同じarchiveへ含める。
- identityとoptional secretは保存backendに依存せず論理値として読み出す。keyring名やfile fallback名はarchive formatへ露出させない。
- `iroh-data`は停止後に再帰列挙するが、端末固有の`endpoint-secret.json`は除外する。

### 5. 復元と競合

- 復元は専用staging directoryで全entryの認証、長さ、hash、path、公開鍵、SQLite migrationを検証してから行う。
- 公開鍵が未登録なら新規アカウントとして追加する。同じ公開鍵が登録済みなら対象を表示し、明示的な置換確認がある場合だけアカウントdirectory全体を置換する。内容単位のmergeは行わない。
- 置換前directoryはrollback用に退避し、新runtimeの構築とregistry更新が成功した後に削除する。失敗時は旧directory、identity、optional secret、registryへ戻す。
- 復元成功時は対象アカウントをactiveにする。frontend stateは明示したkeyだけを復元し、再読込後に適用する。

### 6. 設定画面のフロー

- 対象利用者は端末故障への備え、または別端末への移行を行うdesktop利用者とする。単一目的は、鍵だけの移行と端末全体の移行を混同せず、安全に1アカウントを持ち出すことである。
- 作成は説明・秘密情報警告・確認checkbox・パスフレーズと確認入力・native保存先選択・進捗・cancel・成功／失敗を持つ。復元はnativeファイル選択・パスフレーズ・内容preview・任意設定の適用・既存アカウント置換確認・進捗・cancel・失敗回復を持つ。
- 狭幅では1columnを維持し、長いpathと公開鍵は折り返す。pointerとkeyboardの同じcontrolを使い、native dialog以外の操作にdragやhoverを必須としない。
- offlineでもローカルファイルの作成・preview・復元は可能とする。runtime停止中のネットワーク同期は再起動後に再開し、remote copyを削除したような表示はしない。
- 非目標は複数アカウント一括選択、バックアップ内容の個別編集、クラウド同期、旧端末の遠隔削除である。

## Consequences

- データ分類は`docs/legal/device-backup-data-classification.md`を正とする。
- raw iroh storeは同一の現行desktop世代でのみ復元対象とし、外側のbackup formatとは独立したcomponent versionを持つ。非対応component版は既存状態を変更せず拒否する。
- 初版は複数アカウント一括archive、クラウド保管、定期実行、内容mergeを提供しない。

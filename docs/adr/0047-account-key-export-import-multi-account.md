# ADR 0047: アカウント鍵の暗号化エクスポート・インポートと複数アカウント

## Status
Accepted

## Context
- Issue #859(Parent #853 Phase A)。秘密鍵の紛失で本人性を復旧できない・端末変更時にアカウントを移行できない状態は、無償 Preview 公開のブロッカーである。中央集権的な「ログイン」ではなく、鍵による本人性の移行として設計する。
- 本 ADR の対象は本人性を表すアカウント鍵(secp256k1、`KukuriKeys`)に限定する。ローカル DB・添付・設定・非公開チャネル秘密等を含む完全な端末移行/バックアップは #855 の領分であり、対象範囲の違いを UI と文書の双方で明示する。
- 従来の識別子ストレージは「db path につき 1 identity」(keyring `db:<canonical path>` / `<db>.identity-key`)を前提としており、複数アカウントの概念が存在しなかった。
- インポートは再起動での反映ではなく「アカウントの追加」として扱い、複数鍵の管理・切替を同時に導入する(ユーザー決定)。後方互換より単一レイアウトへの統一を優先する(破壊的変更可、ユーザー決定)。

## Decision

### 1. エクスポート形式(`kukuri-account-key.v1.<base64url(JSON)>`)
- 暗号化: XChaCha20-Poly1305。鍵導出は argon2id(パラメータは envelope に記録、インポート時は DoS 防止の上限内でそのまま使用)。パスフレーズは 8 文字以上を必須とする。
- envelope(JSON)は `version` / `kdf` / KDF パラメータ / `salt_hex` / `nonce_hex` / `public_key`(fingerprint)/ `ciphertext_hex` を持つ。**version・KDF パラメータ・salt・public_key は AEAD の AAD に束縛**し、改竄・バージョン偽装を認証エラーで検出する(整合性検証)。
- `preview` はパスフレーズなしで version / kdf / fingerprint のみ返し、復号は行わない。復号後は導出 pubkey と envelope の `public_key` の一致を検証する。
- 平文秘密鍵は UI・IPC・ログ・クリップボード・診断レポートのどこにも出さない。IPC に載るのは暗号化 envelope のみで、パスフレーズを含む request DTO は `Debug` を `<redacted>` にする。
- 実装: `crates/core/src/identity_export.rs`(純粋・I/O なし)。

### 2. アカウント毎データディレクトリと registry
- ストレージは `<app_data>/accounts/<account_id>/kukuri.db` の単一レイアウトに統一する。`account_id` は公開鍵 hex の先頭 16 文字(安定・衝突実質なし)。
- db path から導出される既存の per-identity 状態(identity keyring/file、CN 設定・トークン、非公開チャネル capability、gossip 購読状態、iroh データ、content-display 設定)は、この変更だけで**すべて自動的にアカウント毎**になる。単一 SQLite DB の複数アカウント共有は行わない(`dm_conversations.peer_pubkey UNIQUE` 等が衝突するため)。
- アカウントの列挙と `active_account_id` は `<app_data>/accounts.json`(registry)で管理する。registry は**秘密情報を含まないメタデータのみ**(id / pubkey / label / timestamps)を持ち、既存の temp→fsync→rename プリミティブで原子的に書く。
- 端末レベルの状態(アプリ同意・年齢自己申告 `<app_data>/kukuri.app-consent.json`、テーマ/言語、OS 通知設定)は base ディレクトリに残し、アカウントに紐づけない(アカウント追加/切替で再同意を発生させない)。
- 旧 flat レイアウト(`<app_data>/kukuri.db`)は初回起動時に一括移行し、以後サポートしない。移行順序は「鍵を新レイアウトへ複製→検証→registry 書き込み(commit point)→ファイル移動・keyring 再登録→旧実体削除」で、どの時点でクラッシュしても鍵が最低ひとつの場所から読める。registry 書き込み後の残骸は次回起動時に冪等に再開する。
- 実装: `crates/desktop-runtime/src/accounts.rs`。

### 3. インポート = アカウント追加(原子的反映)
- インポートは復号・検証をメモリ上で完了させ、新しいアカウントディレクトリと identity を作り切ってから registry に追記する。既存アカウントには一切触れないため、途中失敗で既存状態は壊れない(残骸ディレクトリは再実行で上書き)。
- 同一 pubkey が登録済みの場合はエラーで拒否する(既存鍵の in-place 置換は存在しない。preview の `already_registered` で事前に UI 警告する)。
- インポート前に fingerprint(公開鍵)を preview で確認できる。

### 4. 再起動なしのアカウント切替
- `DesktopState` は `RwLock<Arc<DesktopRuntime>>` を持ち、切替コマンドは「新 db path で runtime を構築→成功後に registry の `active_account_id` 更新→Arc スワップ→旧 runtime `shutdown()`」の順で行う。構築失敗時は旧 runtime が無傷で残る。切替は guard mutex で直列化する。
- 旧 runtime のイベント購読チャネルは閉じるため、通知ディスパッチタスクは再 subscribe し、OS 通知の pubkey キャッシュ / dispatch cursor はリセットする。`DesktopStartupState` を Initializing→Ready と駆動して frontend を再同期する。
- frontend は切替完了後に列下書き(localStorage)を破棄して再読み込みし、新アカウントの状態で UI を再構築する。ワークスペースレイアウトは端末レベルの表示設定として共有のままとする。

### 5. マルチ端末・競合・失効/ローテーションの制約(v1)
- 同一鍵の複数端末利用: 各端末が独立に投稿・購読する。投稿は署名済み envelope としてマージされ衝突しないが、端末間で DB・下書き・非公開チャネル capability は同期されない(#855 の領分)。同時利用による本人性の競合は発生しない(同一鍵 = 同一本人)。
- 失効・ローテーション: v1 では提供しない。鍵が漏えいした場合、その鍵を無効化する中央機構は存在せず、なりすましを止められない。エクスポート前警告でこのリスクと「運営者は復旧できない」ことを明示する。鍵ローテーション(新鍵への本人性移行の告知)は将来の別 issue の領分。
- パスフレーズ喪失 = エクスポートの復元不能。運営者を含む誰にも復元できない。

## Consequences
- ADR 0002 分類は `docs/legal/account-key-export-data-classification.md` に定める。
- `KUKURI_INSTANCE` / `KUKURI_APP_DATA_DIR` で指定したディレクトリ配下も同じ accounts レイアウトになる(dev runbook の手順は据え置き。ディレクトリ構造だけが 1 段深くなる)。
- 「アカウントを識別する鍵はユーザーの端末にのみ保存される」という法務文言は維持される(エクスポートはユーザー自身の明示操作であり、アプリが外部へ送信することはない)。`LEGAL_BUNDLE_VERSION` は変更しない。
- `.nsec` legacy 読込経路は per-account ディレクトリ配下でも従来どおり機能し、その sunset 条件(REFACTORING.md)は本 ADR で変更しない。

# #1091 動画抽出の一時失敗と readiness の失敗表示

## 状態と固定範囲

- 区分C、Scope revision `2026-09-17 r1`、基準commit `503bead6`。AC-1〜5、INVAR-1〜5、INV-1〜6、TR-1〜7 は [Issue #1091](https://github.com/kukuri-app/kukuri/issues/1091) に固定した。
- 現在判定・独立監査・CI・merge照合は Issue と PR の記録を参照する。
- 非対象: 本番VMへの反映と本番での確認（本番反映をまとめる別Issue）、抽出設定値・`EXTRACTOR_VERSION`・fingerprint の変更、OpenAI呼び出しと共有予算の仕様変更。

## 原因

| 事象 | 原因 | 根拠 |
| --- | --- | --- |
| main Fast（run 35180988522）の `bundled_readiness_decodes_both_containers` の busy 失敗 | `JobDirectory::create` が root lock（`flock`）を即時失敗の `try_lock` で取る。解放済みの lock も、並行するテストが fork した子processが exec まで fd を複製して保持するため、`new()` 直後の `extract` が busy になる | 同binary内で既定rootを使うのは当該テストだけ。修正前は、子process起動を続ける4 threadと `extract` 2000回で 769〜1019回 busy、起動なしの対照は0回。テスト一式30回反復で CI と同一の失敗も1回再現 |
| `cancel_reaps_decoder_and_removes_temporary_data` の `child must be reaped` | shellのリダイレクトが `child.pid` を作ってから書くため、空読みで pid が空文字になり `/proc/` を検査していた | 作成と書き込みの間に0.2秒待つscriptで、修正前は決定的に失敗、修正後は成功 |
| 本番 readiness の初回失敗（2026-09-17） | 未確定。予算予約0行から decoder 段階の失敗と判断。初回 ffprobe 2.39秒（以後0.12秒）の計測から、cold start が probe 期限5秒に掛かる可能性が最も高い。ただし旧実装は ffprobe / ffmpeg の時間切れを同じ文言で返し、readiness は理由を捨てていた | 初回execだけprobe期限より遅いffprobe wrapperで、修正前は `Timeout("video decoder deadline exceeded")` |

本番でのプロセス間共有は Compose / Terraform では起きない（containerごとの専用tmpfs）。
直接起動でindexerとreadinessが `/dev/shm/kukuri-video` を共有する構成と、同一process内の並行する子process起動では同じ一時失敗が起こり得た。

## 変更

- `workdir`: root lock を最大2秒、10ms間隔で再試行する。起動時（`new`）はblocking、抽出時はasyncで待つ。上限超過だけ busy。
- `lib`: extractorごとに最初の抽出前に `ffprobe -version` を同じ隔離環境・decode期限で実行する（準備）。起動後のprobe時間切れは準備をやり直して1回だけ再試行する。`VideoExtractConfig`・`EXTRACTOR_VERSION`・fingerprintは不変。
- `failure`（新規）: crateが所有する固定文言を `DecoderFailure` へ分類する `classify_failure`。`process::run` は時間切れ理由（準備 / probe / decode）を呼出元から受ける。
- `cn-cli readiness_openai`: 失敗段階と分類だけで detail を作る。HTTP status は `moderation HTTP NNN` から3桁の数値だけ取り出す。未知の文言は分類外として出し、エラー文字列を転記しない。
- テスト: extraction テストは全て専用rootを使う。lock保持の待機・上限、fork負荷、cold start、準備の時間切れ、遅い入力の上限、readiness の段階・分類・OpenAI未使用を追加。
- 運用手順: [OpenAI Moderationの運用](../runbooks/community-node-openai-moderation.md) に失敗表示と一時失敗の扱いを追記。

## 検証

Linux は Docker（`rust:1.92.0-bookworm` + Debian ffmpeg）で実行した。CI の linux-cn も同じテストを実行する。

| 対象 | 修正前 | 修正後 |
| --- | --- | --- |
| `concurrent_process_spawns_do_not_report_scratch_busy` | 失敗（busy 769〜1019/2000） | 成功 |
| `short_scratch_maintenance_is_waited_for` | 失敗（即時 busy） | 成功 |
| `cold_decoder_start_does_not_consume_the_probe_deadline` | 失敗（`Timeout("video decoder deadline exceeded")`） | 成功 |
| `cancel_reaps...`（pid書き込み遅延版） | 失敗（`child must be reaped`） | 成功 |
| extraction テスト一式30回反復 | 毎回上記3件が失敗し、既存テストでも busy 1回・decoder未起動1回 | 追加テストの補助関数を修正後、下記の反復で失敗0 |
| `cargo test -p kukuri-cn-safety-video` | — | 13 + 2 件成功 |
| `cargo test -p kukuri-cn-cli --bin cn-cli readiness` | — | 14件成功（Linux。追加の段階・分類・OpenAI未使用テストを含む） |
| `cargo xtask cn-check` / `cn-test`（Windows） | — | 両方成功（Windowsでは Linux 限定テストは対象外） |

修正後の反復: テスト一式40回で失敗0。`cargo clippy -p kukuri-cn-safety-video -p kukuri-cn-cli --all-targets -D warnings`（Linux）成功。

## 引き継ぎ（本番確認）

次回の本番反映で `cn-cli readiness --force-probe` を image 更新直後に実行し、general が PASS すること、失敗した場合は detail の段階と分類を記録する。
本番確認は [#1093](https://github.com/kukuri-app/kukuri/issues/1093) で行う。

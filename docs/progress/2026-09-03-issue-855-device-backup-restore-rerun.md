# Issue #855 device backup / restore rerun

## Summary

無償 Preview 公開のブロッカー(#853 Phase A)である端末バックアップ・復元について、再監査で不足していた失敗契約と実データ復元 scenario を補強した。既存の ADR 0048 と v1 archive format、公開 API、ユーザー向け挙動は変更していない。今回の再実行では、容量枯渇・flush 失敗・キャンセル・既存出力先・破損/切詰め/未知 version を実装境界で再現し、失敗時に部分出力を残さず、既存アカウント registry と app data を変更しないことを固定した。

意図的にやらなかったこと: 未知の将来 version を推測して移行する処理、backup format の変更、restore による Community Node の bearer token・同意状態・年齢自己申告・成人向け表示許可の引継ぎ。v1 以外は fail-closed で拒否し、端末固有または再同意が必要な状態は従来どおり除外する。

## 実装内容

- core archive contract (`crates/core/src/tests/device_backup.rs`): 容量枯渇 writer と flush 失敗を使い、I/O error の伝搬を固定。未知の archive format version と component version、切詰め、末尾データ、弱い passphrase、invalid manifest の拒否を網羅した。
- desktop runtime failure contract (`crates/desktop-runtime/src/backup.rs`、`backup/create.rs`、`tests/device_backup.rs`): production では `File` に透過する内部 writer wrapper を追加し、test 時だけ deterministic な write budget で ENOSPC 相当を注入できるようにした。backup/restore の容量枯渇、キャンセル、既存 destination、誤 passphrase、破損、切詰め、未知 version、replace 拒否で、部分出力 cleanup と既存 app data の byte-for-byte 不変を検証する。新規テストの共有 IdentityStorage lock は lock classification contract に宣言した。
- 実データ scenario (`crates/harness/src/scenarios/device_backup.rs`): offline で画像添付投稿、下書き、bookmark、mute、block、private channel、Community Node 設定を作成して runtime を停止後に backup。空の別 app data へ restore し、投稿本文、添付 metadata と blob bytes、下書き、bookmark、mute/block、private channel、Node 設定が戻ることを実 API で検証する。endpoint secret と成人向け表示許可が戻らないことも確認する。bearer token と Node 同意の除外は desktop runtime contract で固定し、アプリ同意と年齢自己申告の非移行は既存の Tauri state contract を維持する。

## 検証

- `cargo test -p kukuri-core device_backup -- --nocapture`: 7 件成功
- `cargo test -p kukuri-desktop-runtime tests::device_backup -- --nocapture`: 7 件成功
- `cargo test -p kukuri-desktop-runtime tests::support::lock_contract::lock_acquisitions_match_declared_classification -- --nocapture`: 成功
- `cargo test -p kukuri-harness -- --nocapture`: 22 件成功
- `cargo xtask scenario desktop_device_backup_restore`: 成功
- `cargo xtask check`: 成功
- `cargo xtask test`: Rust 748 件、harness 22 件、frontend 1074 件すべて成功(Rust 3 件 skip)
- `cargo xtask rust-test`: Rust 748 件、harness 22 件、doc-test すべて成功(Rust 3 件 skip)
- `cargo xtask e2e-smoke`: `desktop_smoke_post_persist` 成功
- `cargo xtask doctor`: 成功

関連: #853(親)、#855、#859(アカウント鍵 export/import)、ADR 0048

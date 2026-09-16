# #1063 windows-fast の Windows package timeout の作業記録

## 現在の状態

- 判定: Complete。リスク区分 A、Scope revision: 2026-09-16、基準 commit: `773e52c3`、merge commit: `1c45b232`（PR #1074）。
- 2026-09-16 にユーザーが推奨案（step timeout 45 分、repo 全体の cache 予算は別 Issue）を承認した。Issue 操作・commit・PR 作成・CI 成功後の merge は承認待ちなしで行う。
- AC / INVAR の正本は [Issue #1063](https://github.com/kukuri-app/kukuri/issues/1063)。Linux job を含む cache 予算は [#1073](https://github.com/kukuri-app/kukuri/issues/1073) に分離した。

## 修正前の観測（2026-09-14〜16 の `Kukuri Fast` 29 run）

`windows-fast` の step 所要時間を attempt 単位で集計した。

| 状態 | Windows package | 該当 |
| --- | --- | --- |
| cache なし | 23.0〜25.0 分、または 25 分 timeout | main の全 run、PR の初回 run |
| sccache 部分 hit（53〜72%） | 15.0〜17.3 分 | timeout 後の `rerun --failed` |
| rust-cache 完全 hit | 8.0〜11.4 分 | 同一 PR の直前 run から間を置かない再 push（2 run のみ） |

cache なしの内訳（main run 35073909006）は vite build 約 1 分、依存 594 crate の release compile 約 16.5 分、workspace 12 crate 約 7.5 分、NSIS 約 0.7 分。

cache が効かない原因:

- repo の Actions cache が 14.5 GB / 3642 entry で 10 GB 上限を超えており、entry が 1〜4 時間で追い出される。内訳は rust-cache 約 6.5 GB、sccache 約 4.0 GB、`kukuri-cn-images` の buildkit blob 約 3.3 GB。
- main の `Cache Rust` は Windows 8 run・Linux 16 job のすべてで「No cache found」。PR scope の entry は main から読めない。
- Windows の sccache は初回 run で hit 0%、write error が job あたり 354〜2082 件。Windows は 1 job なので同一 run 内でも共有されない。

## 対応

| 条件 | 変更（`.github/workflows/kukuri-fast.yml` の `windows-fast`） |
| --- | --- |
| AC-1 | `Windows package` の `timeout-minutes` を 25 から 45 へ。70% は 31.5 分で、cache なし実測の上限を上回る |
| AC-1 | rust-cache に `cache-on-failure: true` と `save-if: main のみ` を追加。timeout した attempt の依存 build も保存し、PR run は main の entry を読むだけにする |
| AC-1 | job env で `RUSTC_WRAPPER: ''`・`SCCACHE_GHA_ENABLED: 'false'` とし、`Setup sccache` と `Show sccache stats` を削除。初回 run で効かない sccache の書き込みを止める。cargo 1.92 が空の `RUSTC_WRAPPER` を wrapper なしとして扱うことはローカルで確認した |
| AC-2 | job 上限 75 分は据え置き。観測最悪値の見積もりは setup と doctor 12 分、tauri-check 9.5 分、package 31 分、cache 保存 6 分で約 60 分 |
| INVAR-1 / INVAR-2 | 他の step、順序、timeout、artifact 名・path・upload 条件は変更なし（差分で確認） |

検証: `actionlint .github/workflows/kukuri-fast.yml`、`git diff --check`。

## CI 計測（AC-1 / AC-2）

基準は Windows package が 31.5 分以内（timeout 45 分の 70%）、`windows-fast` 全体が 75 分以内。全 run とも GitHub-hosted の `windows-latest`。

| run | commit | Windows package | windows-fast 全体 | rust-cache | 判定 |
| --- | --- | --- | --- | --- | --- |
| 35108080793（PR #1074） | `d01acc0f` | 23.8 分 | 40.8 分 | miss、`save-if: false` で保存なし | 基準内 |
| 35112821418（main） | `1c45b232` | 22.1 分 | 41.5 分 | miss、main scope に保存 | 基準内 |
| 35115095739（main） | `0fc0b03f` | 20.5 分 | 37.9 分 | miss（開始が保存より前） | 基準内 |
| 35119088717（main） | `b8357c35` | - | - | - | repository 移管時に run 全体が cancelled。計測対象外 |
| 35131926193（main） | `e25a8686` | 20.8 分 | 43.5 分 | miss（key は一致。移管前の entry は復元されず、この run で再保存） | 基準内 |

判定: main の連続する有効 3 run で AC-1 / AC-2 を満たした。INVAR-1 / INVAR-2 は PR #1074 の差分で確認済み。

- sccache を外しても cache なしの package は 20〜24 分で、修正前（23〜25 分）より遅くなっていない。
- main で保存した rust-cache の復元効果はこの 3 run では観測できなかった。repo 全体の cache 予算と runner 移行は [#1073](https://github.com/kukuri-app/kukuri/issues/1073) で扱う。
- 2026-09-16 に repository を `kukuri-app/kukuri` へ移管した。本記録のリンクは移管後の名前に揃えた。

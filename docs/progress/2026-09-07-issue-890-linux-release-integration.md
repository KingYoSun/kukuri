# #890 Linux Release統合の作業記録

## 現在の判定

- 実装中。Scope revision `2026-09-07-issue-890-linux-release-integration-v2`、区分C。
- 基準commit `fe156251e400ea6d0bc83384683bdd0b343b5e57`。承認済みのコミット・PR・CI／独立監査後マージまで進める。実Releaseのversion／tag／公開範囲は未確定で、公開済みとはしない。
- 条件・INV-1〜8／TR-1〜7は[Issue #890](https://github.com/KingYoSun/kukuri/issues/890)。#889の実機・CI・監査を再利用し、全OS連携の再検証はしない。

## 変更境界と修正前の事実

| 対象 | 現行入口 → sink・不足 | 対応 |
| --- | --- | --- |
| INV-1／3 | releaseのtag／dispatch文字列がinline shellへ展開されてから検証される。後続checkoutも可変tagを再解決 | 環境変数で入力し、検証したsource SHAを後続へ固定。無効／untrusted入力の拒否tests |
| INV-2／4 | release buildはWindowsのみ。#889の別workflowにLinux AppImage生成あり。CLIの配布archiveは未接続 | 既存package入口を再利用し、CLI2archの配布binary smokeを追加 |
| INV-5／6 | `create-preview-assets.ps1`はWindows拡張子を選択し、検証前に出力先へコピーする。manifestはWindows1entry | 完全な入力一覧・source／hash／署名sidecarを検証後に集約。失敗時の既存出力不変・publish禁止 |
| INV-7 | published signature scriptは固定repositoryのWindows entryのみ | Linux entryと最終候補の検証を追加。更新エンジンは変更しない |
| INV-8 | release／利用文書はLinux source-only、旧手動確認一覧が残る | 配布実装と確認済み範囲へ同期し、CLI例の確認はarch smokeと兼用 |

新規の製品機能・保存形式・プロトコル・署名方式は追加しない。sign／publish権限、未検証artifactの混在防止、公開notice／source提供条件を今回の監査対象とする。

## 実装と有限な証拠対応

| 条件／surface／transition | 実装・検査 |
| --- | --- |
| AC-4・8、INVAR-2・3、INV-1・3、TR-2・4 | `release_assets.release_input`と`test_release_workflow`。入力を環境変数経由にし、tag／event検証後だけsource固定・secretを必要jobへ渡す。fork PRの配布secret非供給とpublish依存guardを検査 |
| AC-1〜4、INVAR-1・2、INV-2・4、TR-1・3 | Linux既存packageを再利用、`kukuri-cli-package.yml`／`cli_archive.py`で2archのELF判定・archive・schema／status／shutdown。GUI runtime／保存形式／依存は基準commitから変更なし |
| AC-1・6・8、INVAR-2、INV-5・6、TR-1・5 | `release_assets.py`／集約PowerShellで4targetのversion／source／hash／同一公開鍵／配布署名modeを要求。欠落・改変・混在・duplicate拒否、集約失敗時の既存出力不変。`publish_preview`は事前検査前のAPI呼出し0、upload失敗時draft保持、同一assetのみ再開 |
| AC-1、INV-5、TR-5 | `native_compliance.py`は外側runtime bytes、fixed source／patch／notice、exact Ubuntu sourceとDSCを検査。`test_native_compliance`は改変runtime／source拒否を確認。提供物と由来はruntime-evidence runbookに追記 |
| AC-5・6、INVAR-1・4、INV-7、TR-6 | staging wrapperでWindows／Linux両manifest entry、URL／checksum／exact 1 testを検査。実Rust verifierは配布bundle受理と1 byte改変拒否。`verify_public_preview`は安定URLと4本体／metadata hashを照合、stale latestで本体download 0 |
| AC-7、INVAR-4、INV-8、TR-7 | README en／ja、quickstart／dev／release／troubleshooting／linux-cliを同期。archiveにCLI手順を同梱し、例のschema／status／foreground終了をarch smokeと共用 |

入口／sinkの逆引きはrelease workflowのjob依存、reusable workflowの全callsite、`release_assets`のpackage／plan／validate-output、`publish_preview.publish`のGitHub API、wrapperのcargo／verifier実行、CLI smokeのsubprocessを対象にした。inventoryは8 groupを維持し、製品route追加0。独立監査前の自己確認で未分類0、監査判定は別途記録する。

## 検証結果（PR前）

- Windows／WSLのPython fixturesとPowerShell集約2種は成功。追加wrapper fixtureは両platform正常、外部URL／query／fragment／hash不一致／0件test拒否と環境復元が成功。public URL fixtureは正常と本体改変・stale latest拒否が成功。
- 実Windows／Linuxの#889 bundleをそれぞれ対応する公開鍵でRust verifierへ渡し、正常受理・1 byte改変拒否が成功。両fixtureは異なるtest鍵であり、配布候補へ混在させていない。最終同一配布鍵での検査はRelease workflowが所有する。
- x86_64 CLIをWSLでrelease buildし、実archiveからversion／専用daemon ready・status／schema／SIGTERM終了・socket cleanupが成功。aarch64はPR CIで補完し、現時点では未確認。
- static source／notice計31件の実downloadとhash照合、および#889 AppImageのruntime prefix照合が成功。使い捨てUbuntu 22.04 containerで94 Ubuntu source packageを取得し、304 files／454863735 bytesのDSC size／hash照合が成功。ホストAPT／profileは変更していない。
- `cargo xtask e2e-smoke`: Windowsで成功（desktop_smoke_post_persist、6 steps）。`cargo xtask tauri-check`も成功。workflow parserはWSLのPyYAMLで3 tests成功。
- 同一配布候補のfull native collector、配布secret署名、全asset公開・安定URL確認は未実施。上記部品検査や旧test署名を公開完了の証拠へ読み替えない。

ローカル証跡は`test-results/kukuri/issue-890-cli-x86_64`、`issue-890-static-source-verification`、`issue-890-ubuntu-source-check/verified`。一時成果物・秘密鍵はcommit対象外。

## 残る工程

固定PR headの独立監査と必須CI（aarch64実行を含む）、merge tree照合。その後に公開version／tag／source／公開範囲を確定し、最終候補の配布署名・集約・公開結果を確認する。AC-1・5・6の最終公開証拠が揃うまでIssueはOpenとし、親#885を自動Closeしない。

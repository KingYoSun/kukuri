# #890 Linux Release統合の作業記録

## 現在の判定

- 実装・公開の検証完了。現Scope revision `2026-09-07-issue-890-linux-release-integration-v4`、区分C。以下の実装時の記録は当時の結果として保持する。
- 実装merge `c4616fc706b94150ac6c2ac06aec68bc1c2b0f5a`（PR #904）、Deb追加`016b91588a1a82eb86f19b63397e9d6fedd94b62`（PR #906）。公開source `af2cf56b52e1d2802ac92af6b99090e260dfc48d`から[v0.2.0-preview.2](https://github.com/KingYoSun/kukuri/releases/tag/v0.2.0-preview.2)をlatest公開し、WindowsとLinux4本体を含む全21資材を確認した。
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

## PR #904の独立監査

対象`633ecd183bf182c602991a9cc690cf6039d54b30`、base／Scope revisionは上記のまま。2担当がIssueの条件から独立再構築し、実装時の結論を前提にせず確認した。

- 公開／署名／集約: PR実装PASS。担当7 groupは適合7・不適合0・未分類0。Python13 tests、PowerShell集約／wrapperに加え、upload digest不一致・upload中tag移動・公開済み不完全Releaseでも公開PATCH 0回を確認。blocker 0。
- native source／CLI／文書: 実装コードPASS。static31件、Ubuntu94 source packages／303構成filesを独立再照合し、runtime inventoryとの差集合0。x86_64実archive smoke成功。固定headのrelease runbookに非実在command名1件があり、文書を含む判定はFAIL（6 group中適合5・不適合1・未分類0）。両OS共通の`cargo xtask desktop-package`へ統一する1行deltaを独立確認しPASS、修正後は適合6・不適合0・未分類0。
- 監査後の変更は上記文書1行と本記録のみ。コードsurfaceは固定headと同じ。後続headの差分一致をPR commentへ記録する。CI未完了・aarch64実行未確認・実配布署名／公開未実施は留保し、Issue完了PASSとは区別する。

## 最終公開証拠（v0.2.0-preview.2）

詳細は[全体公開・VM記録](2026-09-07-v0.2.0-preview.1-release-rollout.md)。旧留保のうち、最終署名・native source・aarch64実行・公開は次で解消した。

| 条件／inventory／transition | 最終証拠 |
| --- | --- |
| AC-1・6、INVAR-1・2、INV-2/5/6/7、TR-1/6 | Release `384062770`、全21files、5本体、Windows／AppImage／Debの3manifest entries。source・version・hash・鍵一致、公開後stable manifestと5本体／checksum／provenanceの実download照合PASS |
| AC-2〜4、INVAR-4、INV-2/4/8、TR-1/3/7 | Release run `34109294504`のLinux製品検証、Windows／Linux package、CLI x86_64／aarch64の実archive・schema／専用daemon smoke成功。AppImage／Deb更新・保持は#889/#905の不変surface証拠を採用 |
| AC-1・5・6、INV-5/7、TR-5/6 | 同runの実verifierで3形式の正常bytes受理と1byte改変拒否。Ubuntu92 exact sourcesの297files、static31filesの実archiveとhashを独立照合。Deb payloadと外側runtimeの対応もPASS |
| AC-4・8、全INVAR、INV-1/3/5/6、TR-2/4/5 | 既存入力／secret／欠落・改変／禁止publish tests維持。Fast `34109293103`全9jobs PASS、PR #909と#911はCI・独立監査PASS。集約smokeの終了値誤判定だけを同source／同runで再実行し、全検証成功前に公開していない。元Release CIのFAILは保持 |
| AC-7、INV-8、TR-7 | README en／ja・quickstart・Linux CLIを実公開状態へ同期。CLI専用profile／foreground開始・status・schema・終了の例と未確認OS条件は維持 |

最終候補の独立監査は6群すべて適合、不適合0／未分類0／blocker0。親#885の条件はこの公開だけで自動完了とせず、子の実装・監査・承認済みsupport範囲を別途対応付ける。

## 親 #885 の統合証拠

親条件を子のCloseだけで判定せず、別担当が各最終headとmergeのtree一致、公開sourceの祖先であること、以下の契約・検証を独立確認した。親INV-1〜5／TR-1〜7は未分類0、具体的な製品・公開blocker0。

| 親条件 | 子・実装・採用証拠 |
| --- | --- |
| AC-1 | #886／PR #897、merge `41e97089`。共通host、profile単一owner、同意前非起動、restart／shutdown境界 |
| AC-2 | #887／PR #898、merge `6f89fae0`のprotocol／安全なI/Oと、#888／PR #901、merge `a1c2696b`の承認済み台帳撤去・1入力1実行。旧idempotency台帳を現条件へ戻さない |
| AC-3 | #888／PR #901。131対象と理由付き除外、78tests、実CLI／複数daemon／public・private・DM・Live・Game・Domeの独立監査 |
| AC-4 | #889／PR #903、merge `fe156251`。Ubuntu22.04 build、Ubuntu24.04代表実機、自動境界tests。追加Ubuntu22.04／Debian12実機・XWaylandは2026-09-06利用者回答と#889 AC-2の延期を維持。Debは追加承認された#905／PR #906を採用 |
| AC-5 | #890／PR #904、Deb PR #906、配布修正PR #909と公開source af2、今回の全21assetsと公開後検査 |
| INVAR-1〜3 | #886〜888のconsent／secret／profile／audience／P2P監査、#889／905の更新保持、#890のWindows・公開境界 |

親本文の旧未完了表示・全追加OS実機条件・Deb対象外表記は、既承認の子条件と追加依頼に同期する。support保証の追加や品質条件の免除ではない。最終docsと本文同期のdelta監査後にCloseを判定する。

# #916 再オープン対応の独立監査

- 判定: **PASS**
- 対象PR: [#969](https://github.com/KingYoSun/kukuri/pull/969)
- 監査対象の製品・test head: `df35d7f6ebaba3ea8c7040e5e6e63446f1beb802`
- 基準: `f74243c3c2001d5c257e67ca5c77d9b01de0091f`
- Scope revision: `916-r1`、再開計画 `916-reopen-20260910`、リスク区分 C
- inventory: 合計6 / 適合6 / 不適合0 / 未分類0
- concrete blocker: 0
- 監査日: 2026-09-10

固定条件は[AC-1〜5 / INVAR-1〜4 / INV-1〜6 / TR-1〜8 / SINK-1〜3](2026-09-09-916-consent-gate-affordance.md)を使用した。実装担当とは別の監査担当が、source・test・実行ログ・実機画像とartifactを照合した。旧監査の成功判断やIssue checkboxは根拠にしていない。今回の予備監査で再構築した不変経路は、最終headとの差分0を確認して利用し、全testを重複実行していない。

## 対象とartifactの固定

`gh pr view 969 --json headRefOid,baseRefOid`で上記head/baseを確認した。監査開始時のworktree差分はprogressとUI証拠画像・JSONのみで、製品・testはheadに一致していた。

Ubuntu24の通常AppImage候補は、[package run 34443470080](https://github.com/KingYoSun/kukuri/actions/runs/34443470080)由来のtest signing artifact。公開Releaseへ署名・配布された版とは区別する。

- ファイル: `.codex/plans/issue-916-reopen/package-b0f00db5/kukuri_0.2.1_amd64.AppImage`
- 独立計算したSHA-256: `a0a2825d3d95b3307ae8ca70a53b718e569ff7b1097371ff976dc1c1100ea2f3`
- `source-commit.txt` / `release-package.json`のsource: `e6018dbc922567c0bb7952e75cfa125b6779523b`
- `appimage-artifacts.json` / `release-package.json`のhashと実ファイルhashが一致。
- 合成merge `e6018dbc` と製品変更commit `b0f00db50beea285623829dc0f9623e9f4eff708` は全tree差分0。
- `b0f00db5→df35d7f6` はvisual.specと同意画面PNG3個だけ。製品treeは同一。
- runtime inventoryのAppImage同梱WebKitは `2.50.4-0ubuntu0.22.04.1`。実zoom試験のhost WebKit2.52.6と区別した。

Windows実行fileは `apps/desktop/src-tauri/target/debug/kukuri-desktop-tauri.exe`。SHA-256 `824b57af9194be74e9e48766e380ae6601633658888ed1cbef6e1a0611d9204a`を独立計算し、稼働PID25784のExecutablePathと一致させた。子PID26120のWebView2実行pathは `152.0.4191.66`。試験profile `.codex/plans/issue-916-reopen/windows-profile`を直接列挙し、marker / lockのみで同意file・accountがないことを確認した。

## 入口とsinkの独立再構築

CodeGraphの `ConsentGate` / `LegalDocumentView` / `acceptAppConsents` / `record_app_consents` / `save_app_consent_store` / `spawn_desktop_initialization` / `applyPendingDeviceRestoreFrontendState` を起点とし、登録点・文字列IPC・CSS・別名importは限定したrgで補完した。CodeGraphの依存概要だけでcaller数を確定していない。

| ID | 入口 → helper → sink / guard | 分類・対応TR |
| --- | --- | --- |
| INV-1 | App startup/reload → getDesktopStartupStatus。initializing時のみ再取得。ready時のみrestore frontend apply。Tauri setupはrestore_startup_actionで未同意をAwaitConsentへ倒しruntime初期化を延期 | 適合。TR-1/6/7 |
| INV-2 | ConsentGateView → LegalDocumentView / LocaleSelect / Button。checkboxはReact state、拒否はnotice、localeは既存locale保存。native disabledとdescriptionを維持 | 適合。TR-1〜5。locale保存を同意保存と混同しない |
| INV-3 | handleAcceptの年齢guard / acceptInFlight → acceptAppConsents → local accept_app_consents。表示文書版・言語・年齢boolは不変 | 適合。TR-2〜4/6 |
| INV-4 | resolve/reject → onAccepted / error。ready時だけapplyPendingDeviceRestoreFrontendState、apply成功ならreload。error後の選択保持、明示retry、initializing/failed分岐は不変 | 適合。TR-4/7 |
| INV-5 | Tauri文書検証 → switch lock → running / ConsentRequired guard → restore phase検査 → record_app_consents。年齢拒否は保存前return。保存後に初期化またはrestore activation | 適合。TR-1/6/7 |
| INV-6 | AboutPanel / startup error / CN / LocaleSelect / Button consumer、CLI session・直接受諾。共有helper / token / Rust productionの差分0 | 適合。TR-8 |

SINK-1: `record_app_consents`のproduction callerはTauri commandとCLI `ClientSession::accept_consents`。さらに保存helperを逆引きすると、CLI `main.rs::accept_consents`の直接保存と `reset_app_consent_at_path`が存在する。CLI直接受諾は両明示flagと非空language、ProfileLeaseに支配される不変のINV-6 consumer。resetは復元時のINV-5/TR-7で、同意付与ではなく既存同意の解除である。保存先は `db_path.with_extension("app-consent.json")`。

SINK-2: `build_desktop_state→ClientHost::start_if_consented`がaccount/runtime構築前に同意を再検査する。startup / 受諾 / restore rollbackのcallerを確認した。未同意のUI mockだけでnetwork未開始を立証せず、source上の構築前return、host test、invoke guardのprotected hit 0を組み合わせた。

SINK-3: Tauri startup / 受諾 → orchestrate_restore_activation / activate_pending_restore、別consumerはCLI session。frontend applyのproduction callerはApp startup readyと受諾readyの2箇所。restoreのCommitted reset、AwaitingConsent、保存成功後activation、rollback / finish-forwardを既存状態表と照合した。

Buttonの48 import元は `rg -n 'ui/button' apps/desktop/src` で列挙し、別名importなし。CSS変更は `.app-consent-actions .button:disabled` のみで、pending時の拒否buttonにも適用される。gate ancestor外のstartup error・AboutPanel・CN・通常shellへ漏れない。法務本文・版・metadata、共有Button、global token、AppのIPC/status処理、Rust productionはいずれも基準から差分0。production入口・sinkの追加削除0。

## AC / INVAR と証拠

| 条件 | 独立確認した証拠・判定 |
| --- | --- |
| AC-1 | Viewの常時理由文とcheckbox/buttonのaria-describedbyを確認。App「consent explains the missing age confirmation before any action」、browser locale matrix、実機未選択画像で操作前の理由を確認。適合 |
| AC-2 | 未選択は状態名＋鍵icon・中立背景・破線・影なし、選択後は通常action名＋primary。native disabled / aria-busyは維持。新browser6条件はtoken解決後の実背景・background-image・影・境界・状態名、無効pointer後と再解除後をassert。Ubuntu24 AppImageとWindows通常Tauriのbefore/after画像を直接閲覧し識別可能と判定。pendingは既存処理中文言。適合 |
| AC-3 | 1280×800実配布画像でcheckbox/理由/両操作が可視、別画像で文書末尾を確認。既存3viewport×3locale×2themeと390×541 notice累積testを維持。WebKit実zoom2.0のsource/JSON/画像を照合し末尾とfocus到達、横overflowなしを確認。適合 |
| AC-4 | AppのIPC0 / pending抑止 / retry、browser pointer/Space/Enter/touch、実機keyboardと再解除の画像・記録。新browserは無効clickとチェックだけでIPC0をassert。適合 |
| AC-5 | ja/en/zh-CN翻訳キー、48 Story状態のa11y違反0、12contrast条件、locale/parity/保存失敗retry/既存初期locale testを確認。文字contrast最小4.7859、無効境界内側4.6896/外側5.2854。適合 |
| INVAR-1 | 現行版年齢はcheckbox不要、旧版は必要というApp分岐が不変。renewed browser、旧版/remount App、Rust年齢判定・legacy記録test。適合 |
| INVAR-2 | 年齢/pending guard、保存前Rust拒否、構築前host gate、invoke hit0。実機未保存の選択が再起動で初期化される画像、専用profile観測。拒否・scroll・checkboxはaccept IPCを発行しない。適合 |
| INVAR-3 | 文書全文/metadata/参考訳/更新/拒否とIPC payload、ready/initializing/failedおよびrestore frontend applyは不変。LegalDocumentView/App/deviceBackup/Rust restore test。成人向け表示への変更なし。適合 |
| INVAR-4 | scoped selectorとcaller逆引き・差分0。既存assertを削除せず、状態名追従のlocator変更は新6条件のaccessible name assertで補完。視覚比較は対象だけ強化、共有token/他baselineは不変。適合 |

TR-1/2/3は未選択→無効click→選択→解除とIPC0、TR-4はpending→rejection→明示retry、TR-5は拒否/未保存選択→再起動、TR-6は現行/旧版年齢と文書更新、TR-7は欠落/破損/restore各phase、TR-8は不変consumerと既存test/visualで対応付けた。401・CN再認証・複数nodeのglobal apply・background consent retryはこのgateに存在しないため対象外。

## 閲覧した検証と監査担当の実行

監査担当が実行したのはCodeGraph / rg / git差分、PR・run status読取り、file hash、Windows process/profile列挙、JSON集計と画像閲覧。以下の製品testは実装時・CIのログを読み、test sourceと照合したもので、監査担当が再実行したとは扱わない。local logのrootは `.codex/plans/issue-916-reopen/`。

| 証拠 | 確認内容 |
| --- | --- |
| affordance-red.log / affordance-green.log | 変更前「年齢確認が必要」に対し「同意して続行」でfail。変更後新6＋既存26の32 passed |
| linux-desktop-ui-check.log | lint/typecheck/Storybookを含む各構成gate完走。App14、LegalDocumentView9、deviceBackup5、parity57を含むVitest165 files/1314 passed、browser152、visual20 passed |
| host-boundaries.log | targeted7 passed、211はfilter非選択。保存不変/欠落破損/guard/host/restore phase・rollback |
| ci-linux-rust-tests.log | 926 passed/既存4 skipped、harness22 passed、doctest。CLIの両明示確認と保存test、restore reset永続化testも成功 |
| ci-package.log | Tauri lib57 passed。invoke同意前拒否/locale protected hit0/restore frontend allowlist/年齢/文書/legacy/終了待機のnamed testを確認 |
| story-a11y.json / contrast.json | 8state×3locale×2themeの48件、違反数合計0を独立集計。contrast12件の最小値を集計 |
| 最終head CI | `gh pr checks 969`で11 checksすべてpass。[main CI](https://github.com/KingYoSun/kukuri/actions/runs/34444093428)、[package CI](https://github.com/KingYoSun/kukuri/actions/runs/34444093457) |

代表的な境界testは `missing_age_attestation_does_not_create_or_mutate_consent_file`、`consent_acceptance_is_only_allowed_from_consent_required`、`missing_or_invalid_consent_is_fail_closed`、`consent_gate_does_not_initialize_account_or_runtime`、`startup_action_covers_every_safe_restore_boundary`、`restore_consent_reset_is_persisted_before_consent_required_status`、`consent_reset_failure_leaves_committed_phase_unchanged`、`accept_activation_success_and_phase_write_failure_use_the_same_rollback_boundary`。host未構築testの直接assertはConsentRequiredとaccounts.json不存在であり、packet captureではない。

視覚比較は `visual.spec.ts` の全画面対象を0.001、同じbuttonのblocked/readyの領域比較を0.001へ強化。既存global0.01でbutton変更が見逃された記録と整合し、変更PNGは当該3個だけ。Linux CIで比較が有効なことはPlaywright設定 `ignoreSnapshots: !process.env.CI` とCI成功から確認した。Windowsの比較skipを視覚回帰PASSとして数えない。

## 実機証拠と限界

[未選択](../ui-reviews/assets/issue-916-reopen-after-unchecked.png)・[選択後](../ui-reviews/assets/issue-916-reopen-after-checked.png)・[文書末尾](../ui-reviews/assets/issue-916-reopen-after-document-end.png)と公開版before画像を直接閲覧した。公開版は無効色自体が適用されていたため、CSS未適用を原因と断定せず、通常action名・実線輪郭の識別性不足というExisting-gapとして整合する。今回の状態名・破線・鍵iconと有効primaryの違いは画像上も成立する。

Windowsの[未選択](../ui-reviews/assets/issue-916-reopen-windows-blocked.png)と[選択/focus](../ui-reviews/assets/issue-916-reopen-windows-checked.png)を閲覧した。通常Tauriの実入力記録、live process/hash/profile確認を合わせる。

WebKit実200%は[JSON](../ui-reviews/assets/issue-916-reopen-webkit-zoom.json)と画像、`webkit-zoom.py` / `native-init.js`を確認。set_zoom_level/get_zoom_level2.0、DOM640×400、末尾4661+190=4851、横幅628、選択focus下端365、解除後396。failuresは空、同意IPC0。初期表示で操作がviewport外でもpage scroll/focusで到達するため、200%では到達性を判定し1280×800の初期可視条件と混同しない。この試験はhost WebKit2.52.6＋synthetic IPCであり、AppImageの同梱engineでの実zoom確認とは呼ばない。

通常AppImageの追加境界観測では、`capture-native-state.py`と `native-state-denied-initial.json` / `native-state-denied-after-actions.json` / `native-state-accepted.json`を読んだ。監査担当もSSH経由で稼働PID1020738の `APPIMAGE` / `APPDIR` / `KUKURI_APP_DATA_DIR` / executableを照合し、実行AppImageのhash/sourceを再確認した。無効click→拒否後の専用 `profile-audit-denied`はmarker/lockのみで、同意file・accounts.jsonは存在しなかった。collectorの初期/操作後は同じPID・profile・artifactを示す。一方、明示受諾済みの別profileは文書記録2・年齢申告1・accounts.jsonがあり、journalの06:45:49 UTCでruntime初期化、対応RDP画像で通常CN案内への遷移を確認した。監査担当自身がGUI入力を再実行したのではなく、保存された入力観測・画像と直接のprocess/filesystem観測を組み合わせた。

`native-package-journal.log`には初回/再起動後の同意待ちと、その後の明示受諾時の初期化がある。選択済み→再起動未選択→受諾後のraw RDP3画像も直接閲覧した。初期profileの過去不存在を受諾後の現在状態から推測せず、追加の未受諾profileで確認を補った。

拒否noticeまで確定した追加証拠は `native-state-denied-after-notice.json`（06:52:26 UTC）と `package-denied-notice-rdp.png`。noticeと未選択・無効buttonの同時表示を画像で確認し、その後のSSH直接列挙でも同じprofileがmarker/lockのみであることを再確認した。Windowsの直接観測は保存済み `windows-native-state.json` とも一致する。

## blockerとnon-blocker

- Existing-gap AC-2: 今回の修正・同条件の実配布物確認・回帰testで解消。新たなRegression/Existing-gapは発見せず、blocker0。
- Windows `cargo xtask test` は543 passed/1 failed/358未実行/既存4 skipped。失敗は `transport_custom_relay_bootstrap_seed_reports_relay_supported_p2p` のRelaySupportedP2p期待に対するRelayFallback。firewall表示の観測だけで原因を断定しない。同一testのLinux CI成功を直接確認し、今回transport/Rust変更0、対象境界host7/Tauri57と最終CIで補完。Windows全体成功とは記録しない。今回diffに起因するRegressionの証拠はなく、本IssueのClose blockerには分類しない。
- 強制service再起動中のFUSE停止と同時のSIGBUSは正常終了成功の証拠にしない。未保存checkboxが新プロセスで初期化される観測と、通常受諾後にshellへ進む観測を分ける。
- 報告者の元の実行fileのhash、音声screen reader実発話、OS High Contrast、全locale/themeのAppImage実機直積、AppImage同梱engineでの実200%は未確認。公開同版の再測定、browser/Story matrix、通常AppImage実画面、独立WebKit実zoomの確認範囲を越えて成功を主張しない。これらだけから固定条件違反または今回Regressionが立証されたものではない。
- 新規一般化・無効click toast・同意条件変更等を追加完了条件にしていない。

このPASSは上記製品・test headに対する判定。docs/media追記後は対象surface差分0と記録の整合だけをdelta照合する。merge後のtree一致・Issue現在判定更新は後続工程であり、本記録の作成時点でmerge完了とは扱わない。

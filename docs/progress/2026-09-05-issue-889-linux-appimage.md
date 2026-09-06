# #889 Linux AppImage実装記録

## 現在の判定

- 状態: 実装中。通知クリック修正版までの起動・終了・保持・通知操作は実機確認済み。外部リンク対応版のUbuntu起動で見つかったnested runtime panicは依存feature変更後の自動再現testと実機再起動で解消を確認。互換性修正版の生成・署名検証・Linux release全43 testに加え、Firefoxで最新リリースが開くことと元投稿・返信2件の保持を実機確認した。描画残像はprocess再起動後に再現せず原因未確定。全体の実機検証・独立監査は未完了。
- リスク区分: C。Scope revision: `2026-09-07-issue-889-linux-appimage-v5`。v2の環境延期、v3のWindows通知復帰、v4の共通投稿スクロールを維持。v5では本人の承認により追加の網羅的Computer Use検証を打ち切り、成功済み実機証跡と変更影響に絞った自動検証を採用する。製品AC／INVARと署名・同意・identity・データ保護は維持する。
- 基準commit: `d2414e5a58ca14a8920f588164df8efad8e7ffba`。
- 2026-09-05に計画全体を承認。Tauri updater署名のみとし、GPG署名は追加しない。
- 実機証跡の環境はUbuntu 24.04.4 LTS／GNOME 46／Waylandと既存Windows。Ubuntuのシステム情報上のGPUはAMD Radeon Graphicsだが、実描画経路・driver／device転送は未確認。追加Ubuntu 22.04／Debian 12・XWaylandは延期済み。v5ではComputer Useを既定／必須手段とせず、全OS連携・device／GPUの手動確認を実行ゲートから外す。未確認・延期を成功や動作保証に読み替えない。
- 2026-09-07にClose可能と判断できるまでの続行と、コミット・PR作成・CI成功後マージを明示承認された。必須CIと独立監査PASS、マージ後確認を経てCloseする。#890のRelease集約・公開は対象外。
- v4の共通投稿スクロールは実装・全体自動検証1175件とbrowser suiteが成功し、Windows／Linuxの新しい検証用成果物・署名照合まで完了。両OSの実機でアプリ内通知・OS通知から個別投稿へのスクロールが成功し、同じアプリ内通知の再クリック・通常更新時の位置保持も確認した。LinuxのOS経路はGNOME通知センターからのクリックで確認した。Issue全体の残条件・独立監査は別途残る。
- Windows installed版0.1.8の基本操作・LANでの投稿同期・実再起動後の保持を確認。最初のinstallerの隔離配置を本人のExplorer再インストールで是正し、通常のexe配置とStart menu登録を確認した。追加承認に基づくprotocol activation修正版でウィンドウ復帰を確認した後、WindowsのURI正規化により対象移動が拒否される不具合を再現・修正。Windows42件・frontend全1160件等、新しいNSISの生成と署名検証が成功した。正規形対応版PID74944では、本人による通知センター／バナー両方のクリックから同processの再表示と正しいthreadの選択・表示が実機で成功した。v4の追加承認を受け、通知元の個別投稿までの共通自動スクロールを実装し検証中。topic join timeoutによるリアルタイム接続の未復帰も残る。
- 2026-09-06追加検証: 診断export、clipboard、single instance／hidden restore、SIGTERM→同profile再起動・3投稿保持をUbuntuで確認。別workspaceの依存notice欠落を修正し生成／Check成功。先の更新pairの異常系拒否・正常更新に続き、更新表示修正版でも404案内、「あとで」／手動確認／panel再表示後の適用待ち保持、実行file置換・正常終了・新版再起動を実機確認した。15:01時点は新版PID186747が同じ保存先で通常表示、試験serverは停止済み。定期checkを跨ぐ待機、置換不能、全データの詳細比較等は残る。Windows NSIS生成・実Tauri署名検査・本人によるinstallが完了し、専用profileでのsmokeを実施中。

## 条件と対象

### 検証範囲の見直し（2026-09-07承認、v5）

本人がComputer Useの遅さ、権限制約、反復操作による時間・トークン・手作業のコストを指摘し、変更影響へ絞った検証への計画修正を依頼した。これは製品の安全条件の免除ではなく、検証方法と今回の実行ゲートの変更である。

- 既存の実機成功は成果物／source／対象経路に対応付けて採用し、対象変更や具体的な失敗なしに再実行しない。通知2入口の両OS成功を維持し、Linuxバナー再クリックの追加投稿は要求しない。
- 終了競合・tray／サービス不在・鍵取得失敗は制御fixture／隔離サービス、定期確認は実scheduler入口と制御時計、UI状態はDOM／browserで不足を補う。実署名verifier・filesystem置換・process停止・保存状態比較は必要な実OS資源を使い、mockだけの成功と区別する。
- 未変更のmic／camera、全GPU／codec、全リンク・全OS連携の手動確認は今回の実行対象外。同梱構成・配布条件と変更による影響は引き続き確認する。未確認機能を利用可能と断定しない。
- 追加手動実機は、固定条件に関わる具体的な問題を既存証跡と自動検証では判定できない場合だけ、対象・不足理由・最小操作・終了条件をまとめて相談する。Computer Useの使用や再配置・再buildを自動的な前提にしない。操作権限の迂回や汎用GUI基盤の新設はしない。
- named inventory、禁止副作用の確認、必須CI、独立監査は維持。過去節の「未実施」「次は実機」は当時の記録であり、現在の残条件は末尾と承認済み計画のv5検証対応による。

### 通知元投稿への自動スクロールの追加範囲（2026-09-06承認、v4）

本人がOS通知・アプリ内通知の両方への追加を承認。UI分類は既存画面の改善。既存の投稿focus表示を利用し、カラム構造・色・通常の閲覧操作を再設計しない。T4／T6へ次を追加する。

| ID | 条件／状態遷移 | 入口・副作用・検証 |
| --- | --- | --- |
| AC-NS-1 | OS通知／アプリ内の投稿通知を開くと、正しいthreadを表示し、通知のobject_idの投稿を表示範囲へ移す | Windows URI／Linux native event／通知行 → 共通handler → thread取得／投稿focus → 選択中Columnの縦scroll。handler・DOM・browser・実機 |
| AC-NS-2 | 初期pageにない対象は同threadの追加page取得後に解決する。欠落／取得失敗／cursor停滞で別投稿を誤ってtargetにせず、終端または回復可能なerrorへ移る | same topic／threadのlistThreadだけを使用。遅延・追加page・欠落・失敗・cursor反復tests |
| AC-NS-3 | 同じ通知の再クリックは再度移動できる一方、refresh／追加pageだけでは位置を奪わない。古い取得結果で最新の移動を上書きしない | 明示操作の一時request state、cleanup／取消、frame待ち。再クリック・連続クリック・refresh・他Columntests |
| INVAR-NS-1 | 指定topic／channel／accountの範囲を維持し、他Columnのdraft・縦scroll、通知既読、本文preview、DM／follow導線を変更しない | 既存通知ID照合とscope指定を維持。thread内の存在確認後だけfocus。禁止navigation／mark-read、独立Columnと既存通知tests |

スクロール要求はsession内のLocal Only状態。新しいIPC、OS通知引数、DB／backup項目、peer送信は追加しない。scope確認前の実機状態と基準commitの共通handlerは末尾の切り分け記録をbefore証跡とする。

### Windows通知クリック復帰の追加範囲（2026-09-06承認）

Scope revisionを `2026-09-06-issue-889-linux-appimage-v3` とする。ユーザーがクリック復帰の調査・修正の追加を承認。v2の延期条件を維持し、既存AC／INVARを変更せず次を追加する。UI分類は描画不変の不具合修正。

| ID | 条件／状態遷移 | 入口・副作用・検証 |
| --- | --- | --- |
| AC-WN-1 | Windowsの新規通知を常駐中にバナー／通知センターからクリックすると、同じprocessを再表示し該当通知の対象を開く | background／明示通知 → Windows XML → OS protocol activation → single-instance／deep-link → 既存通知hook。実機の修正前失敗・修正後成功、XMLとhookのtests |
| AC-WN-2 | 同期前の通知一覧、初期URIとlive eventの競合、連続クリックでも最新の対象を解決し、重複subscriptionや余分なnavigationを起こさない | 初期URL取得／onOpenUrl／既存native event → pending ID → 現在の通知一覧。listener cleanupと遅延解決tests |
| INVAR-WN-1 | 不正URI・未知ID・別accountのIDを根拠にprofile切替、招待import、任意URL／program実行、mark-readを行わない | 厳密なnotification URI parserと既存NotificationViewのID照合。拒否時navigation／mark-read 0回のtests |
| INVAR-WN-2 | 本文・秘密値・招待token・保存先をactivation URIへ含めず、quiet／previewとLinux通知経路を維持する | Windows通知builderは通知IDだけをURLへ格納。XMLのescaping／引数とLinux既存通知tests。NSISは自アプリshortcutのstub CLSIDだけを追加 |

既存のReady画面と通知target handlerを再利用し、Column配置・draft・scope・themeを再設計しない。COMサーバー、通知入力欄、別profileの自動探索、P2P形式変更は追加しない。各要件はT4・T6に対応し、PR／merge／独立監査の既存条件は維持する。

受入条件・維持条件・対象一覧・状態遷移は [#889](https://github.com/KingYoSun/kukuri/issues/889) のAC-1〜6、INVAR-1〜3、INV-1〜6、TR-1〜7を使用する。データ分類と署名方針は [ADR 0049](../adr/0049-linux-gui-cli-control-plane.md) に反映する。

| 対象 | 登録入口・呼出し元 → 処理 → 副作用先 | 確認する条件 |
| --- | --- | --- |
| INV-1 | `xtask/src/main.rs::main` → `desktop_package` → `run_pnpm` → `tauri-cli.mjs::main` → Tauri CLI／bundler／signer | TR-1・2。Linux署名必須、Windows既存動作、host／target、鍵と成果物 |
| INV-2 | Linux設定 → Tauri bundler → AppImage／icon／desktop entry／依存library | TR-1・3。Ubuntu 22.04、metadata、deep-link登録、codec／license |
| INV-3 | `lib.rs::run` setup／single-instance／deep-link → `spawn_desktop_initialization` → profile lease／runtime | TR-3。同意・復元、二重起動、cold／warm URI |
| INV-4 | close／tray Quit／signal／logout／update → 終了処理 → host／DB／Iroh／process | TR-4・6。trayの到達可能性、終了順、起動・復元との競合 |
| INV-5 | OS通知command／background poll、clipboard／URL／download／dialog／media／WebGL、identity | TR-5。サービス・device不在／拒否、安全な表示、秘密値・既存鍵の保持 |
| INV-6 | update store caller → updater plugin → verifier／AppImage置換／再起動 | TR-7。正常更新、不正署名・取得／置換失敗、旧版とデータの保持 |

CodeGraphで登録入口と既存helperを確認した。INV-3〜6の詳細memberと逆引きは実装する各作業で補完し、現段階で未分類0・全件適合とは判定しない。

### 終了guardのnamed inventory補足（2026-09-06）

CodeGraphのcurrent sourceと `require_running`／`switch_guard` の該当path検索で、終了処理の直接callerと排他後の再確認点を照合した。以下はsource対応であり、未実施のnative競合sequenceを成功扱いするものではない。

| INV／TR | 名前付き入口・共有処理 | guardと副作用 | 対応証跡 |
| --- | --- | --- | --- |
| INV-4／TR-4 | `lib::build_tray` Quit、`RunEvent::ExitRequested` → `shutdown_and_exit` → `request_exit` | `DesktopLifecycle::begin` が重複停止を抑止し、`drain` がswitch_guardを取得後、現在のhostを停止してからexit | `exit_waits_for_initialization_or_restore_before_stopping_latest_runtime`、実機Quit／logoutの前節 |
| INV-4／TR-4 | `WindowEvent::CloseRequested` → `close_window` | 作成済み＋実登録済みtrayだけhide。不在／確認失敗は共通exit。非同期照会後にもrequestedを再確認 | `close_only_hides_when_tray_was_created_and_has_a_registered_host`、GNOME登録書式test、実機hide／restore |
| INV-4／TR-4 | `watch_signals` のSIGTERM／SIGINT／SIGHUP → `request_exit` | setup中にhandlerを登録。停止完了後にexit | 実機SIGTERM／logout。全signal×起動状態の実機直積は未実施 |
| INV-4／TR-6 | setup初期化task、`accept_app_consents`、`switch_account`、`create_device_backup_command`、`restore_device_backup_command` | switch_guard取得後にrequire_running。終了待ち中に取得した操作によるruntime生成・復元開始を拒否 | `operation_queued_before_exit_does_not_recreate_runtime`。実AppHandleでの全競合確認とは区別 |
| INV-4・5／TR-5・6 | `background_notifications::spawn` のaction listener、`poll_once` | switch_guard取得後に終了guard。終了開始後の通知操作を進めない | source照合と既存background tests。UI通知クリックは前節の実機証跡 |
| INV-4／TR-6 | `invoke_gate::with_desktop_startup_gate` | app command本体前に終了gate。終了中はstatus読取りとbackup cancelだけを維持 | `exit_rejects_new_operations_but_keeps_backup_cancellation_available` |
| INV-6／TR-7 | `useAppUpdateStore::restartAndInstall` → `restart_after_update` → `request_exit(Restart)` | 検証済み更新の明示install成功後だけLinux再起動IPC。backendでもReady／終了gateを適用 | updater store tests、実AppImageの停止完了→新版起動ログ |

## 修正前の再現と検証

- packageの既存host選択を引数で検査できる関数へ切り出し、Windowsの既存引数を保持したうえでLinux生成と設定の契約testを先に追加。
- 修正前の `cargo test -p xtask package_tests -- --nocapture`: 3成功・2失敗。Linux生成がWindows専用errorで拒否されること、Linux設定が存在しないことを再現した。
- Linux専用設定とx86_64 targetを追加。鍵欠落時は生成前に拒否する。media frameworkの追加bundleは無効とし、ホスト依存と実際のbundle内容・licenseを実生成後に検査する。
- 実機の画面・tray・GPU・更新の成功を静的確認から推定しない。

### 初回package確認

- 修正後の `cargo test -p xtask package_tests -- --nocapture`: 5成功。
- `cargo test -p xtask -- --nocapture`: 45成功、実AppImageを要求する1testは通常実行ではignore。その後、生成した実ファイルで同testを明示実行し1成功。正常署名の受理、1 byte改変・別公開鍵・署名欠落の拒否を確認。
- WSL Ubuntu 22.04、x86_64、Node 22.14.0、pnpm 10.16.1、Rust 1.92.0。元workspaceを変更しない分離作業領域へ追跡対象と今回の新規ファイルを複写して生成。公開鍵だけをその複写先で検証用一時鍵へ変更し、本番鍵は使用していない。
- `cargo xtask desktop-package`: 成功。frontend build、Linux release compile、AppImage生成、Tauri署名、xtaskの公開鍵一致検査まで実行。
- 初回成果物: `test-results/kukuri/issue-889-local-package/kukuri_0.1.8_amd64.AppImage`（114,567,672 bytes）。署名、`appimage-artifacts.json`、`test-updater-key.pub`を同じdirectoryへ保存。
- SHA-256: `38d285b9086939ddb241a9124053c9ce2a79ee9c4016953f8bb6df8dc1e5cf7b`。
- この成果物のGUIコードは基準commitのままであり、T3〜T5の修正後成果ではない。tray／終了／更新等のbefore観測に使う。#889全体の完成版や配布用署名済みReleaseと混同しない。
- `file`でELF x86-64を確認。生成desktop entryは `Categories=Network`、`MimeType=x-scheme-handler/kukuri`、icon名を持つ。実機への登録先・起動結果は未確認。
- build hostの `ldd` で未解決libraryがないことを確認。最初の追加確認はWSLへの引数展開が不正だったため破棄し、script file経由で再確認した。これはclean Ubuntu／Debianの依存充足を証明しない。
- `desktop-file-validate` はWSLに未導入のため未実行。新規package CIには同ツールの導入と検査を追加したが、workflow自体はまだ未実行。
- `cargo fmt --all -- --check`: module順の差分を修正。workflowは既存 `js-yaml` で構文、Ubuntu 22.04 runner、read-only権限を確認。最終差分に対する再確認結果は後続へ記録する。

### CI・手順の変更

- `.github/workflows/kukuri-linux-package.yml` を追加。PRは一時鍵、明示的manual dispatchのdistributionだけ配布用secretを使う。署名検証済みの現在versionのfileだけを専用directoryへ集め、Releaseを公開せずartifactに保存する。
- `docs/runbooks/linux-appimage-smoke.md` に生成、署名用途、ユーザーのインストールとComputer Useの分担、実機条件・未確認の扱いを記録。開発手順から参照する。

### Ubuntu Desktopでの初回観測とGIO警告（修正前）

- ユーザーが `libfuse2` 導入後、`./kukuri_0.1.8_amd64.AppImage --no-sandbox` を実行し、起動成功と次の警告を報告。Computer Useでも起動中の同意画面と同じ警告を確認した。同意の受諾・年齢申告・profileの変更は行っていない。

  ```text
  /usr/lib/x86_64-linux-gnu/gvfs/libgvfscommon.so: undefined symbol: g_task_set_static_name
  Failed to load module: /usr/lib/x86_64-linux-gnu/gio/modules/libgvfsdbus.so
  ```

- 期待結果: 起動時に互換性のないホスト側GIOモジュールを読み込まず、必要なOS連携が利用できること。警告の非表示だけを修正成功とはしない。INV-2／INV-5、T2／T4の依存構成調査へ統合する。
- 生成したAppDirには `libgio-2.0.so.0.7200.4` があり、生成環境の `libglib2.0-0` は `2.72.4-0ubuntu2.9`。`nm -D` で `g_task_set_name` は存在するが `g_task_set_static_name` は存在しないことを確認。
- `AppRun` が読み込む `apprun-hooks/linuxdeploy-plugin-gtk.sh` は `GIO_EXTRA_MODULES` に同梱モジュールの場所を設定するが、`GIO_MODULE_DIR` は設定していない。同梱GIOモジュールは `libgiognutls.so` で、GVfsは同梱されていない。
- [GLib公式API](https://docs.gtk.org/gio/method.Task.set_static_name.html)によれば不足関数は2.76で導入された。[GIOの公式実行仕様](https://docs.gtk.org/gio/overview.html#running-gio-applications)では `GIO_EXTRA_MODULES` は探索先の追加、`GIO_MODULE_DIR` は組込み探索先の置換である。この構成とホスト側GVfsの未解決シンボルが整合し、単なるモジュール未インストールではなくABI不整合と判断する。実機processの読み込み済みlibrary一覧とホストpackageの正確な版は未採取。
- Computer UseでUbuntuの「システムの詳細」を確認: Ubuntu 24.04.4 LTS、64 bit、GNOME 46、Wayland、AMD Radeon Graphics。これはアプリのXWayland使用や実GPU描画の確認ではない。
- リモート端末への文字入力が届かず、貼り付けも意図した確認コマンドではなかったため、入力を実行せず取り消した。OS情報は設定画面から取得し、端末コマンドによる取得成功とは記録しない。
- `app-level legal consent required; deferring runtime startup` は同意待ちの正常な情報ログ。ユーザーの `^C` は正常終了処理の検証には数えない。追加で見えた `atk-bridge: get_device_events_reply: unknown signature` は別警告として未切分け。
- ホストlibraryの削除・差替え・追加導入や環境変数の恒久変更は行っていない。この観測時点ではAppImageの修正と同条件の再検証は未実施。転送済みファイルの実機側SHA-256も未確認。

### GIO探索先の修正と再生成

- ユーザーからAppImageの同梱構成・読み込み先の修正と再生成を依頼された。INV-2／INV-5、T2／T4の範囲で実施し、CLIや同意・profile・終了処理は変更しない。
- Tauri CLI 2.11.4のbundlerはcustom filesの `usr/` だけをAppDirへ複写するため、AppImage直下の任意hookを追加する案は採用しなかった。最初に置いたhook配置契約testの失敗を確認した後、この案とtestを取り下げ、実際のGIO動作を検証するtestへ置き換えた。
- Linuxの `main` → `appimage_env::configure` をGTK／Tauri／スレッド生成より前へ追加。`APPDIR` がある場合だけ、`GIO_MODULE_DIR` と `GIO_EXTRA_MODULES` を `APPDIR/usr/lib/x86_64-linux-gnu/gio/modules` へ設定する。通常起動とWindowsには適用しない。環境変数の変更は当該processと子processだけで、ホスト設定の変更ではない。
- 同梱済みの `libgiognutls.so` を使い、GVfsや任意のホストGIOモジュールは追加しない。TLSモジュール欠落時はGIO初期化前に明示errorで終了する。ネットワーク共有等のGVfs機能の成功は主張しない。
- `xtask/tests/appimage/gio-isolation.sh` を追加し、package CIから実行する。実際の同梱GIOに対し、新しいシンボルの即時解決を要求するモジュールで修正前の `undefined symbol: g_task_set_static_name` を再現。製品と同じ `appimage_env.rs` を組み込んだ別processで、修正後に不整合モジュールを読み込まず `local=1 tls=1` になることを確認した。空白を含むAppDir、通常起動への非適用、同梱TLS欠落時の拒否も成功。GUI／外部通信／実利用profileは使用していない。
- `cargo test -p xtask`: 45成功・実署名test 1件ignore。`cargo xtask tauri-check`（Windows）: 成功。`cargo xtask e2e-smoke`: 6 step成功。
- `cargo fmt --all -- --check`、変更したRust fileの `rustfmt --check`、shell構文確認、workflow YAML構文とGIO検査の登録確認: 成功。Tauri全体の `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check` は既存の `community_node.rs`／`device_backup.rs`／`lib.rs`／`state.rs` の整形差分で失敗。該当fileは本修正で変更しておらず、無関係な整形は加えていない。
- 再生成はWSL Ubuntu 22.04の別作業領域で実行し、検証用一時鍵で署名する。旧成果物は保持し、修正版の出力先は `test-results/kukuri/issue-889-gio-fixed-package/` とする。実機Ubuntu Desktopでの修正後確認はまだ行っていない。
- 再生成結果: `cargo xtask desktop-package` 成功（Linux release compile 3分04秒、Tauri build全体3分49秒）。修正版 `kukuri_0.1.8_amd64.AppImage` は114,555,384 bytes、SHA-256は `3747af645019ef418cfe05cc1fec7d68b9744d5e6083c98e61e5a67873d37b9c`。`.sig`、`appimage-artifacts.json`、`test-updater-key.pub` を同じ出力先へ保存。
- 新成果物に対する実署名testを明示実行し1成功。正常署名の受理、改変・別公開鍵・空署名の拒否を確認した。最終版のGIO回帰検査も新AppDirで成功。初回成果物とは別の検証用一時鍵のため、旧成果物からのupdater経由更新試験には使わず、直接配置して確認する。

### 修正版のComputer Use確認（2026-09-05 19:37〜19:41 JST）

- ユーザーから修正版の起動成功とComputer Use確認開始の依頼を受け、同じUbuntu Desktopで確認した。期待する成果物hashは上記 `3747af64…73d37b9c`。実機側hashの照合は未実施。
- 起動端末には旧processのログと修正版のログが両方残っていた。`2026-09-05T10:36:18.233612Z` の新しい同意待ちログ以後を区別して確認し、旧版で出ていた `g_task_set_static_name`／`Failed to load module` の警告が新しい起動では出ていないことを確認した。
- `atk-bridge: get_device_events_reply: unknown signature` は新processでも19:36:43に出ていた。旧版でも観測した別の警告として扱い、GIO修正による新規Regressionとは断定しない。アクセシビリティ連携への影響は未確認。
- INV-3／INV-5: 同意画面の日本語表示、最大化後の再描画、文末までのスクロール、年齢checkboxが未選択の状態を確認した。同意・年齢確認は送信していない。画面とログの観測だけから禁止I/Oが0とは断定しない。
- INV-4／TR-4（同意前・トレイあり）: トレイの `Open kukuri`／`Quit` を確認。ウィンドウの閉じるボタンで非表示になり、トレイの `Open kukuri` で同じスクロール位置の同意画面へ復帰した。一時的なGNOMEの準備完了通知の後、追加クリックなしで画面が前面に戻った。Quit／SIGTERM／logout、runtime稼働中の終了は未検証。
- Computer Useスキルの年齢確認代行禁止に従い、年齢checkbox・同意ボタンの操作前で停止。現在の起動コマンドには専用データ領域の指定が見えないため、同意後のデータ操作は行わず、`KUKURI_APP_DATA_DIR` を使う一時領域での起動と本人による年齢確認・同意を依頼する。既存profileの削除・初期化、権限設定変更、投稿は行っていない。

### 同意後のComputer Use確認（2026-09-05 19:44〜19:50 JST）

- ユーザー自身による年齢確認・同意の完了後に再開。端末上で `export KUKURI_APP_DATA_DIR="$(mktemp -d /tmp/kukuri-889-smoke.XXXXXX)"` と、`--no-sandbox` なしの起動を確認した。以後は同じshell環境の起動コマンドを再実行し、一時データ領域を維持した。展開された具体的なdirectory名と実機側hashの照合は未実施。
- INV-3／TR-3: `10:44:15.329012Z` にruntime初期化完了。メイン画面、コントロールセンター、アプリ／法的情報を表示でき、version `0.1.8` とユーザーが行った同意・年齢申告の保存状態を確認。
- INV-4／TR-6（runtime稼働中・トレイあり）: `Quit` でwindowとtrayが消え、起動端末がpromptへ戻った。同じ端末で直前の `./kukuri_0.1.8_amd64.AppImage` を再実行し、`10:47:09.370074Z` に再度runtime初期化完了。再同意なしでメイン画面に入った。DB／Irohの全終了順やidentityの同一性まで検証したとは扱わない。
- INV-3／TR-3: 稼働中のwindowを閉じて非表示にし、Filesから同じAppImageを再実行した。既存画面へ復帰し、既存起動端末に `10:50:02.663463Z` の `received kukuri desktop single-instance activation` を確認。追加windowは観測しなかったが、OSのprocess一覧による厳密なcount検査は未実施。
- INV-5／TR-5: プロフィール編集画面のファイル選択からOSダイアログが開き、ファイル未選択でキャンセルできた。画像の読込み・保存・送信は行っていない。操作後も当該起動のGIO／GVfs警告は観測していない。
- 追加観測: 起動直後のプロフィール概要で「不明なユーザー」「プロフィールを読み込み中…」が残り、再起動でも再現した。編集画面を開き、保存せず概要へ戻ると読込み表示が消えた。`ProfileOverviewPanel.tsx` は渡された `status === 'loading'` で表示するため、未作成プロフィールの単なる固定文言とは扱わない。上流stateの原因・基準commitとの比較・#889内の分類は未確定であり、この確認だけでGIO修正のRegressionとは断定しない。
- `atk-bridge: get_device_events_reply: unknown signature` は同意後の起動と再起動でも観測した。通知機能・アクセシビリティ機能の正常性は、このログやGUI操作成功だけでは判定しない。
- 投稿、プロフィール保存、コミュニティノードの同意・接続変更、権限変更、秘密鍵操作は行っていない。実データを生成する往復、media／device／通知／更新／SIGTERM／logoutの条件は残る。

### 継続作業: 終了・更新・選択欄（2026-09-05）

- 「ユーザーの作業が必要になるまで継続」の依頼に従い、T3／T5と実機で見つかったT4の選択欄を実装した。PR作成・コミット・公開は行っていない。
- プロフィール概要の読込み表示は、`useDesktopShellDataEffects.ts` のeffectと `useDesktopShellSectionLoaders.ts::loadShellSections` が選択中sectionだけを条件にし、非選択のプロフィール列を取得しない経路と整合する。基準commitにも同じ条件があり、今回の差分によるRegressionではない。#889のLinux固有修正へ混ぜず、既存の別件として記録する。`atk-bridge` 警告は未解決で、アクセシビリティを無効化する回避は加えていない。

| 対象 | 変更と入口・副作用先 | 検証・未確認 |
| --- | --- | --- |
| INV-3／INVAR-2・3 | setupの初期化を既存 `switch_guard` と直列化。同意・切替・backup／restore・背景通知の各lock取得後に終了状態を再確認。二重起動ログからargvを除去 | 終了要求後の待機操作がmutationへ進まないunit test。全7箇所のlockと確認呼出しを逆引き。新実機の競合条件は未確認 |
| INV-4／TR-4・6 | `DesktopLifecycle` が終了要求の重複を抑止。Quit／ExitRequested／SIGTERM・SIGINT・SIGHUP／更新後再起動 → 既存操作の終了待ち → 現host.shutdown → process終了・再起動 | 起動／restore待ち中に停止完了を報告しないtest。33件のTauri test成功。全app.exit／request_restart呼出しがこのmoduleへ集約されたことを確認。通常GUI commandの全in-flight I/Oを実機検証済みとはしない |
| INV-4／TR-4 | Linux close時にD-Busのトレイhostと当該PIDの登録を確認。確認不能・2秒timeoutなら正常終了。非表示中だけ2秒間隔で確認し、表示先喪失時にwindowを復帰 | 作成／登録の4組合せtest。Linux compile成功。実GNOMEでの登録照合・表示先喪失は新成果物で未確認 |
| INV-6／TR-7 | 検証済みupdate.install成功後だけ `restart_after_update`。`installing` 表示と操作抑止を追加。Windows installerの実行処理は変更しない | 修正前は再起動要求のtestが1失敗。修正後は順序、install失敗時の再起動0回、署名失敗時のinstall／再起動0回、同時操作1回を確認。実AppImage更新・Windows packaged回帰は未確認 |
| INV-5／TR-5 | 共通Selectの高さ44pxを維持して上下余白を12px→0へ変更 | Ubuntuのdark／lightで言語選択値の上下欠けを修正前に確認。候補一覧は表示でき、Escapeで言語を変更せず閉じた。契約testは修正前1失敗、修正後成功。CodeGraphで11利用元を特定し、表示設定・topic filter・Column専用の指定を確認。Column専用の高さ・余白overrideは保持。修正後の実機は未確認 |

- UI変更の目的・維持項目は承認済みプランのT4／T5と選択欄の追記へ記録した。`ReleasePanel` の「更新を適用中」は日本語・英語・簡体字へ追加し、Storybookの `Installing` を追加。選択欄のtheme切替は実機でdark→light→darkと確認した。
- GUI／CLI対応表は新GUI commandを未分類として失敗することを確認後、GUI専用OS操作として1件追加した。現行基準は `d2414e5a58ca14a8920f588164df8efad8e7ffba`、scopeは#889、GUI登録138件。CLIのcommand／handlerは増やしていない。対応表の先頭行を前提にした既存negative testも維持し、最終 `cargo test -p kukuri-cli --test command_parity` は5成功。
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib`: 33成功。`cargo xtask tauri-check`（Windows）、`cargo xtask e2e-smoke`（6 step）: 成功。`cargo fmt --all -- --check`、新終了module／invoke gateのrustfmt、`git diff --check`: 成功。Tauri全体の既存整形差分は前節のとおり未解消。
- 最終 `cargo xtask desktop-ui-check`: lint、typecheck、Vitest 141 file／1,096 test、Storybook build、browser 58 test、visual操作smoke 14 testが成功。Windowsのvisual実行は画像比較をskipするため、Linuxのpixel baseline照合成功とは扱わない。Select変更前にも一度成功したが、共有Selectを変更したため再実行した。
- 生成中に出たcore APIの動的import警告は、最終版で既存の静的importへ合わせて解消した。既存のVite chunk size、JSDOMの未実装media API、NO_COLOR／FORCE_COLOR等の警告は残る。新たなcompile errorはない。
- 最終成果物: `test-results/kukuri/issue-889-linux-smoke-package/kukuri_0.1.8_amd64.AppImage`、114,874,872 bytes。SHA-256は `d648faa1699321a66cb139cb18d8878a040efbeb77b89ca094e7e1190c8c6463`。WSL Ubuntu 22.04で生成（release compile 3分17秒、Tauri build全体4分05秒）、公開鍵一致検査と実署名negative test 1件成功。同梱GIO回帰検査も `local=1 tls=1` で成功。署名・manifest・検証用公開鍵を同じ出力先へ保存。
- 中間成果物 `issue-889-lifecycle-package/` はSelect修正前であり、実機へ渡す最終成果物ではない。旧成果物は削除していない。各生成は異なる一時鍵のため、これらの組をupdaterの旧新版試験へ転用しない。
- 20:30〜20:31 JSTにComputer UseでUbuntu側の旧成果物をトレイのQuitから終了し、端末がpromptへ戻ったことを確認。端末と一時データ領域の環境変数は維持し、ユーザーへ最終成果物の配置・実行権限付与を依頼する。実機で変更後の終了処理を実行した結果ではない。
- 既存 `src-tauri/tests/updater_signature.rs` はソース側の配布用公開鍵を読むため、この検証用一時鍵の成果物ではまだ実行していない。今回の署名testはxtask側で明示実行したもの。Tauri側の該当testと実更新の証跡は残作業へ保持する。

### 新成果物の実機確認とトレイ判定の修正（21:00〜21:17 JST）

- ユーザーが `issue-889-linux-smoke-package` の配置・実行権限付与を完了後、同じ端末の履歴からAppImageを起動した。`12:00:06.266115Z` にruntime初期化完了し、再同意なしでメイン画面へ到達。
- INV-5／TR-5: コントロールセンターの「すべて」「追加順」、表示設定の「日本語」が完全に読めることを確認。言語選択欄はdark／lightの両方で上下欠けが解消した。確認後はdarkへ戻した。前回までの#889実機確認と同様、OSはUbuntu 24.04.4であり、Ubuntu 22.04／Debian 12の必須確認を代替しない。
- INV-4／TR-4のRegression: トレイが画面上に存在する状態でwindowを閉じると、非表示ではなくprocessが終了した。`12:03:38.573969Z` に `desktop runtime shutdown completed` と端末promptへの復帰を確認。今回追加したトレイ登録判定の不足として扱い、ユーザー環境の設定不備には分類しない。
- [Ubuntu appindicator v58の `indicatorId`](https://github.com/ubuntu/gnome-shell-extension-appindicator/blob/v58/util.js) は `busName@objectPath` を返す。元の `split('/')` ではbus名に末尾の `@` が残り、PID照会が失敗する。`tray_item_service_accepts_gnome_and_kde_registration_formats` を先に追加し、期待 `:1.42` に対して実値 `:1.42@` となる失敗を確認した。
- 修正: `tray_item_service` で `@` と `/` の両方を扱い、GNOME形式、KDE系bus/path、service名だけの形式を同じPID照会へ通す。トレイの利用不能・確認不能時の正常終了方針は変えない。
- 修正後 `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib`: 34成功。`cargo xtask tauri-check`（Windows）と `cargo xtask e2e-smoke`（6 step）、対象Rustの整形、diff検査: 成功。frontendは変更しておらず、前節の1,096 test／browser／visual操作smoke結果を再利用する。
- 端末への `type_text` は通常の貼り付けにならず、次の矢印キーが制御文字として入力された。未実行の入力をCtrl+Cで取消し、D-Busの実機コマンド出力は取得済みと扱わない。以後は確認済みの履歴操作で起動した。
- INV-4／TR-6: トレイparser修正前の同じ配置済みAppImageでSIGINT経路を独立確認。`12:15:27.893195Z` に再起動完了後、起動端末でCtrl+Cを送信。`12:16:25.049536Z` の停止完了ログ、window／tray消失、端末prompt復帰を確認した。SIGTERM／SIGHUP／logoutまで同一結果と推定しない。端末とテスト領域の環境変数はそのまま残している。
- `atk-bridge` 警告はこの再起動でも残る。今回の追加修正はトレイ登録文字列の解釈だけであり、アクセシビリティ機能を無効化していない。
- 再生成結果: `test-results/kukuri/issue-889-tray-fixed-package/kukuri_0.1.8_amd64.AppImage`、114,854,392 bytes、SHA-256 `6b07fe06016326df8d482b3fd420b8740c312f3dd3deb3a180932d8c923cf8ed`。Linux release compile 3分09秒、Tauri build全体3分55秒で成功。同梱GIO回帰検査と署名一致検査に成功し、生成物に対するxtaskの署名negative testも1成功。署名・成果物一覧・検証用公開鍵を同じdirectoryへ保存した。修正後のトレイ格納・復帰は、ユーザーの新成果物配置後に確認する。

### トレイ修正版の実機確認（21:27〜21:37 JST）

- ユーザーによる再配置・実行権限付与後、同じ起動端末から修正版を実行。`12:27:06.050407Z` にruntime初期化完了、同意済み状態でメイン画面へ到達した。
- INV-4／TR-4: windowを閉じてもprocessは終了せず、トレイが残った。非表示状態を維持した後、`Open kukuri` から既存windowへ復帰できた。前節の登録文字列解釈による誤終了は再現しない。トレイ自体の喪失・不在条件は未検証。
- INV-5／TR-5: 空のプロフィール編集欄へ `kukuri-889-local-input` を入力し、選択→コピー→別の空欄へ貼付けで同じ文字列になることを確認。保存・送信はしていない。検査用の文字列も含め、最後にリセット操作で全欄が元の空欄へ戻ったことを確認した。
- 端末では `type_text` の通常貼付けがreadlineの制御文字になるため、GUI欄に一時入力して確認・コピーした読取り専用commandを、検査用の新しい端末タブへCtrl+Shift+Vで貼り付けた。画面上でcommand全文を確認してから実行した。起動元タブと `KUKURI_APP_DATA_DIR` は変更していない。
- 実機で `sha256sum ./kukuri_0.1.8_amd64.AppImage` を実行し、`6b07fe06016326df8d482b3fd420b8740c312f3dd3deb3a180932d8c923cf8ed` を確認。前節の生成済み修正版と一致した。
- `atk-bridge` 警告は起動後も残る。今回の成功は入力・格納・復帰・ハッシュ照合の範囲であり、すべてのOS連携が正常という判定ではない。
- 次の投稿永続化試験は公開トピックへの送信を伴うため、Computer Useの送信時確認に従って承認前に停止する。候補は検証用公開トピック `kukuri-889-smoke-20260905`、本文「Issue #889: AppImageの動作確認用投稿です。」1件。未送信であり、プロフィール保存・投稿・コミュニティノードへの追加同意は行っていない。

## 次の実機観測

`issue-889-tray-fixed-package` で以下の投稿保持・SIGTERM／実desktop logout後の停止・再openを確認した。通知クリック修正版でWindows側の返信受信、OS通知の本文保護、クリックから対象スレッドへの復帰を確認済み。次は描画・残るOS連携を確認する。deep link、media／device、検証用旧新版による更新等も残る。日常profileを使わず、無断の更新や本番データの初期化は行わない。

### 投稿保持とSIGTERMの実機確認（22:04〜22:15 JST）

- ユーザーの「送信して良いです」という承認後、公開トピック `kukuri-889-smoke-20260905` を開き、公開範囲と本文を画面で照合したうえで `Issue #889: AppImageの動作確認用投稿です。` を1件だけ投稿した。投稿画面の時刻は22:04:23。追加投稿・編集・削除・プロフィール保存は行っていない。
- INV-3／TR-3: 投稿がタイムラインへ表示された。端末の `13:04:24.644736Z` の記録ではtopicが一致し、`connectivity_shape="offline"`、`direct_peer_count=0`、`docs_assist_peer_count=0`。この試験はローカル生成・保存の確認であり、他端末への配送やP2P接続成功ではない。
- INV-4／TR-6: トレイのQuit後、`13:04:57.255374Z` に `desktop runtime shutdown completed`、window／tray消失、起動元端末のprompt復帰を確認。同じ端末で同じAppImageを再実行し、`13:05:32.519216Z` にruntime初期化完了。再同意なしで同じトピック・22:04:23の本文1件を表示した。
- SIGTERMを別経路として検証。Linux端末で `ps -C kukuri-desktop-tauri -o pid=,args=` を実行し、対象がPID `37965` の1件であることを確認してから `kill -TERM 37965` を実行。`13:12:09.907666Z` の停止完了ログ、window／tray消失、起動元端末のprompt復帰を確認した。全processを対象とする停止やSIGKILLは使っていない。
- 起動元端末で `printenv KUKURI_APP_DATA_DIR` を確認し、既存の検証用保存先は `/tmp/kukuri-889-smoke.SlPTEA`。環境変数の再設定や新しい一時領域の作成はしていない。22:15 JSTに同じ起動履歴を再実行し、SIGTERM後も同じ投稿1件が表示された。author公開鍵の厳密な同一性、DB／Iroh全データの完全性検査の代替とはしない。
- Linux端末の貼付け制約には、Linux Filesの場所入力欄へ確認文を一時入力し、選択・コピー・Escapeで戻してから端末にCtrl+Shift+Vで貼り付ける方法を使用した。FilesではEnterを押さず、ファイルの作成・変更・移動はしていない。最初に旧clipboard内容が貼られた入力は実行前に取り消し、その後は実行前にcommand全文を照合した。
- `atk-bridge: get_device_events_reply: unknown signature` は残る。logout、トレイ喪失、通知サービス・device権限、実更新の成功へ一般化しない。ログアウト試験はまだ開始していない。

### 実desktop logoutの開始（22:38〜22:42 JST）

- 切断後の本人による再ログイン・再接続を伴う試験としてユーザーから開始承認を受けた。認証画面の代行は行わない。
- ログ採取のため、既存の実行をQuitで終了し、`13:38:41.208321Z` の停止完了ログとprompt復帰を確認。同じ起動元shell・同じ `KUKURI_APP_DATA_DIR` を維持したまま、`mktemp /tmp/kukuri-889-logout.XXXXXX.log` で新しいログを作り、同じAppImageの標準出力・標準errorをそこへ向けて再起動した。新しいprofileや鍵は作成していない。
- 生成されたログpathは `/tmp/kukuri-889-logout.exEfSq.log`。既存ファイルの上書きはしていない。22:40 JSTにメイン画面が開き、22:04:23の同じテスト投稿1件を表示していることを確認した。
- GNOMEのシステムメニュー → 電源メニュー → ログアウト → 確認dialogのログアウトを22:42 JSTに実行。その後、接続先desktopのwindowは利用不能になり、window一覧はリモートデスクトップ接続の初期windowへ変わった。再接続・ログイン操作は行っていない。
- 現時点で確認できたのはlogout操作と接続終了まで。ログの `desktop runtime shutdown completed`、対象processの終了、同じprofileの再openと投稿保持は再ログイン後に確認する。接続終了だけを正常shutdownやTR-6全体の成功と扱わない。

### 再ログイン後の確認（22:47〜22:49 JST）

- ユーザー本人の再ログイン・再接続後、前節のログを読み、`13:40:38.328555Z` の初期化（PID42907）と、logout時の `13:42:09.869093Z` の `desktop runtime shutdown completed` を確認。`ps -C kukuri-desktop-tauri -o pid=,args=` に残存process行はなかった。
- 同ログには22:42:09.852の `GLib-GObject-WARNING: invalid (NULL) pointer instance` と `g_signal_handler_disconnect: assertion 'G_TYPE_CHECK_INSTANCE (instance)' failed` が2組あった。停止完了より前に出ており、既存のatk-bridge警告とは分けて記録する。今回が最初のlogout観測のためRegressionとは断定しない。終了moduleには当該GObject APIの直接呼出しがなく、native依存側を含む発生元は未確定。警告抑制や無根拠なlibrary差替えは行わない。
- `/tmp/kukuri-889-smoke.SlPTEA` が存在し、owner-onlyのdirectoryであることを確認。存在確認を条件に新しいshellへ同じ `KUKURI_APP_DATA_DIR` を設定して起動し、22:49 JSTに再同意なしで22:04:23のテスト投稿1件を表示した。新しいprofileは作成していない。これで提供されたUbuntu 24.04.4環境でのlogout後の停止・再open・ローカル投稿保持を確認したが、全データの完全性や別OSの代替検証とはしない。

### OS通知のサービス判定・確認操作（T4、INV-5／TR-5）

- 既存のget／request permission commandはLinuxでも常に `granted`。通知サービス不在を許可済みとは表示しない、という承認済みT4の不足として修正する。async化だけを準備変更したうえで、別processを無効なsession busへ向けるtestを先に実行し、期待 `unavailable` に対して `granted` となる失敗を確認した。
- [freedesktop通知仕様1.3](https://specifications.freedesktop.org/notification/latest-single/#command-get-server-information)とnotify-rust 4.18.0のsourceを確認。Linuxの両commandは `GetServerInformation` だけを2秒期限で照会し、正常応答は `available`、不在・拒否・不正応答・期限超過は `unavailable` とする。OS側の表示許可・拒否を推定しない。新たな永続保存やOS設定変更はない。
- 画面は `available` を「通知サービスに接続済み（表示はOS設定に依存します）」として3 localeへ追加。取得失敗を `prompt` へ誤変換せず、再確認の失敗も局所表示する。確認中は直前の結果を保持し、ボタンを処理中表示・操作抑止にする。Linuxのサービス確認だけでは通知設定・本文previewを有効化しない。Windowsの `granted` と明示操作後の有効化は維持する。
- frontend修正前: 6 test中5失敗・1成功、明示確認失敗の未処理rejection 1件。修正後の同じ6 testは成功。3 localeを実描画するtestも追加し、最終全体gateで確認する。Storybookにサービス利用可能・利用不可・確認中の3 stateを追加した。
- Linuxの `silent` 引数が無視される既存経路も、通知builderの契約testで失敗を確認したうえで標準 `suppress-sound` hintへ反映。summary／body・default actionは維持し、本文生成と既存のpreview／adult保護には変更を加えていない。実際の音やOS側設定への従属は実機で別に確認する。
- 隔離D-Bus上に検査用の通知サービスを置き、正常→拒否→応答停止→回復を両commandで照会。server情報照会8回、`Notify` 0回を確認した。最初のfixtureはzbus executorからTokio timerを呼んでpanicしたため、その実行の見かけ上の成功は破棄した。応答停止fixtureをruntime非依存に直して再実行した最終Linux Tauri lib testは37成功・panicなし。通常WindowsのTauri lib testは34成功。
- LinuxのtestはWSL Ubuntu 22.04の分離コピー・release profileで実行した。既存の/tmpコピーが残っていなかったため、継続用に `/home/kingyosun/kukuri-889-notification.5WR4xo` を使用。ネイティブUbuntu Desktopへの追加導入・OS通知設定変更は行っていない。
- package CIへ `dbus` とLinux Tauri lib testの実行stepを追加。workflowの実実行、修正版AppImageの実機確認、GLib終了警告の発生元は未確認。

### 通知修正版の最終検証と生成物

- 最終 `cargo xtask desktop-ui-check`: lint、typecheck、142 file／1,104 test、Storybook build、browser 58 test、visual操作smoke 14 testがすべて成功。初回はStorybook mockの拡張子なしpathが解決できず失敗したが、明示的な `.ts` pathへ修正して全gateを再実行した。Windowsでのvisualは画像比較をskipするため、Linux pixel baseline照合成功とは扱わない。
- 通知の3 state × en／ja／zh-CNの390px・darkと、3 state × jaの1280px・lightの12条件を、生成済みStorybookを使う実Chromiumで確認。利用不可から再確認→利用可能へ回復しても通知有効checkboxはoffを維持し、確認中はボタンがdisabledで、ページ例外・対象sectionの横overflowは0。画像は `test-results/kukuri/issue-889-notification-stories/` に保存。日本語の狭幅・利用不可、英語の確認中、中国語の利用可能、日本語lightの利用可能を目視確認し、文字欠け・重なりは観測しなかった。実WebKitGTKの代替ではない。
- `cargo xtask tauri-check`（Windows）、`cargo xtask e2e-smoke`（6 step）、`cargo fmt --all -- --check`、通知moduleのrustfmt、`git diff --check`は成功。workflowはWSLのPyYAMLで構文とnative test stepを確認した。Windows側のNode／PythonではYAML parserが見つからず、そこでの試行を成功扱いしていない。
- 新成果物: `test-results/kukuri/issue-889-notification-fixed-package/kukuri_0.1.8_amd64.AppImage`、114,846,200 bytes。SHA-256 `3a758b355279efd63642d0f439dcbef3990c4d85296a173dc92526e89647e04b`。WSL Ubuntu 22.04の分離コピー `/home/kingyosun/kukuri-889-build.tTxLGS` で生成（release compile 3分10秒、Tauri build全体3分58秒）。GIO回帰検査は `local=1 tls=1`、公開鍵との署名一致検査も成功した。
- 新成果物を使うxtaskの署名negative testは1成功。さらに生成コピーに設定済みの検証用公開鍵を使い、Tauri側の `cargo test --release --target x86_64-unknown-linux-gnu --manifest-path apps/desktop/src-tauri/Cargo.toml --test updater_signature -- --ignored` を明示実行して1成功（正常署名受理・1 byte改変拒否）。テスト名の `published` にかかわらず、この実行は未公開の検証用成果物であり、本番署名済み配布や実updater更新の検証ではない。
- 同じdirectoryに署名・manifest・検証用公開鍵を保存。旧成果物は保持し、今回は新しい一時鍵のため旧成果物とのupdater更新pairには使わない。署名secretは本番のものを使用していない。
- 23:35 JSTにUbuntu側の現在の検証実行をQuitで終了し、`14:35:19.365688Z` の停止完了ログと端末promptへの復帰を確認。端末には `/tmp/kukuri-889-smoke.SlPTEA` を指定した環境を維持している。修正版の配置・実行権限付与をユーザーへ依頼し、配置後に実機確認する。GLib／atk警告の解消は主張しない。

### 通知修正版の実機確認（2026-09-06 00:00〜00:04 JST）

- ユーザーの配置・置換・実行権限付与後、Ubuntu端末の `sha256sum ./kukuri_0.1.8_amd64.AppImage` が `3a758b355279efd63642d0f439dcbef3990c4d85296a173dc92526e89647e04b` と一致することを確認。前節の通知修正版を対象としている。
- INV-3／TR-3: 既存directoryの存在確認を条件に `/tmp/kukuri-889-smoke.SlPTEA` を指定した履歴を再実行し、再同意なしでメイン画面へ到達。前日の22:04:23の検証投稿1件が同じ本文で表示された。追加投稿・編集・削除はしていない。
- INV-5／TR-5: リリース画面のOS通知欄で「状態: 通知サービスに接続済み（表示はOS設定に依存します）」を確認。Linuxの `available` を実WebKitGTKで確認したものであり、OS側の通知表示許可・実配送・音の成功を意味しない。
- 「通知の利用可否を確認」を1回実行し、確認中のボタン文言・操作抑止表示から通常表示への復帰を観測。結果はサービス接続済みを維持し、OS通知有効はOFF、本文プレビューもOFFのままだった。DM／メンションと返信の既存チェック、その他の設定は変更していない。文字欠け・重なりはこの画面では観測しなかった。
- 起動後の `atk-bridge: get_device_events_reply: unknown signature` は引き続き観測。リリース画面上部には更新失敗表示があり、今回の確認をupdater成功として扱わない。GIO ABIエラーの再発は観測していない。
- アプリは同じ検証用環境で起動したままOS通知欄を表示している。実際の通知表示試験には本人による「OS通知を有効にする」の操作が必要。Computer Useのin-app privacy設定操作禁止に従い、代行せずユーザーへ引き継ぐ。本文プレビューやOS設定の変更は依頼していない。

### 通知有効化後の確認（2026-09-06 00:08 JST）

- ユーザーからONにしたとの連絡を受け、Computer Useで「OS通知を有効にする」はON、「本文プレビューを表示」はOFF、通知サービスは接続済みであることを画面確認した。設定操作は代行していない。
- 実通知の発生条件をCodeGraphで確認。`background_notifications.rs::should_send` は自分自身のactionを除外し、`poll_once` は新しい未読通知を対象とする。現在有効な種類はDMとメンション／返信。単なる有効化・自分の検証投稿だけでは実表示を検査できない。
- Release／Developer画面とfrontendの登録済み経路にテスト通知ボタンはなく、`show_os_notification` の直接呼出しを受信経路全体の証拠として代用しない。送信側の別アカウントと実接続が必要なため、利用可能な検証環境をユーザーへ確認する。別アカウントの認証・作成・同意・追加送信はまだ行っていない。
- この時点で実toast・クリック復帰・音・通知設定の再起動後保持は未検証。アプリは同じ検証用profileで起動したまま。

### Windows送信側の起動（2026-09-06 00:13〜00:15 JST）

- ユーザーから、このWindows機のローカルkukuriを `tauri:dev` で起動する指示を受けた。既存のkukuri processがないことと起動scriptを確認し、`apps/desktop` で `npx pnpm@10.16.1 tauri:dev` を実行した。データ保存先・instanceの上書き、新しいaccountの作成、既存データの初期化はしていない。
- Viteは `127.0.0.1:5173`、Tauriのdev buildは1分06秒で成功。`target/debug/kukuri-desktop-tauri.exe`（PID74448）のwindowをComputer Useで表示し、「ご利用の前に」の利用規約・プライバシーポリシー確認画面へ到達した。
- 起動commandは継続実行中。本人による初回確認が必要なため、年齢確認等を代行せず引き継ぐ。main画面到達、Windows側accountとUbuntu側accountの相違、peer接続、返信送信、Ubuntuの実通知はまだ未確認。Ubuntu側の検証実行は停止していない。

### Windows送信側との接続（2026-09-06 00:17〜00:27 JST）

- ユーザー本人が初回確認を済ませた後、Windows dev版のメイン画面を確認。両側とも初期観測はピア0件だった。自動生成のチケットはWindowsが仮想network側、UbuntuがDocker bridge側のIPを選んでおり、そのままLAN到達性があるとは判断しなかった。
- WindowsのLAN interfaceと対象PID74448のUDP待受を読取り専用で確認。endpoint IDとportは実画面の自分のチケットを維持し、hostだけを確認済みのLANアドレスにして双方の「接続先を追加」へ1回ずつ入力した。OS／ファイアウォール／privacy設定は変更せず、community-nodeの追加認証・同意・relay導入もしていない。ticketのendpoint IDの相違をauthor identityの相違の証明とは扱わない。
- Windowsで `kukuri-889-smoke-20260905` を開くと、コントロールセンターが「接続中・Direct P2P」と表示。閉じた後のタイムラインで、Ubuntu側が前日に作成した22:04:23の `Issue #889: AppImageの動作確認用投稿です。` 1件を確認した。Windows側には当該投稿の削除actionが表示されていない。両方向の通知配送・すべてのtopicの接続成功へ一般化しない。
- 次の実通知試験はこの公開投稿への返信1件を予定。候補本文は `Issue #889: Windowsからの通知確認用返信です。`。Computer Useの送信時確認に従いユーザーの承認前に停止する。この時点で返信composerは未入力・未送信であり、既存投稿の編集・削除・追加DMもしていない。Ubuntuは同じ検証用profileで通知ON・本文preview OFFのまま起動中。

### 返信通知の実機確認とクリック時の不足（2026-09-06 00:33〜00:38 JST）

- ユーザーの「OKです」による送信承認後、UbuntuのAppImageをcloseでトレイへ格納し、windowが消えてtray／起動processが残ることを確認した。起動元端末から今回の実行はPID57203、初期化は `2026-09-05T15:01:54.032309Z` と判明した。
- Windows側で元投稿と公開scopeを照合し、`Issue #889: Windowsからの通知確認用返信です。` を添付なしで1回だけ送信。00:35:23の返信1件がタイムラインとスレッドへ表示された。追加送信や編集・削除は行っていない。
- Ubuntuで直後のkukuriバナーは観測できなかったが、00:37 JSTにGNOME通知履歴を開き、`kukuri-desktop-tauri` の `Reply` 1件と本文 `Open kukuri to view this activity.` を確認した。返信のraw本文はOS通知に表示されておらず、preview OFFの保護を実機で確認。音は観測していない。
- 通知本文部分をクリックするとkukuri windowが再表示された。GNOME通知履歴は開いたままで、一時的に「kukuriの準備ができました」というOS側通知が現れた。フォーカス奪取の完全な成功とは扱わない。OS通知履歴を閉じてもアプリのSettingsが残り、Settingsを手動で閉じた後も返信スレッドは開いていなかった。通知Columnには00:35:23の返信1件・未読1件、タイムラインには新着1件の表示があった。
- INV-5／TR-5のExisting-gapとしてクリック経路を調査。非表示中は `useDesktopShellDataEffects.ts::refreshNotificationStatus` が早期returnし、`useOsNotificationActivation` はクリック時のfrontend一覧にidがなければ破棄していた。クリック→一覧更新のhook testを追加し、期待1回に対してcallback0回の失敗を確認。Settings／Control Centerの遮蔽も独立したshell integration test 2件で再現した。
- 修正は未解決クリックidの1件保持・最新クリックでの置換・一覧更新時の1回解決・unmount破棄と、解決後のSettings／Control Center閉鎖に限定。通知内容・送信条件・既読化規則・OS設定・Column draft／scopeは変更しない。既存通常経路を含む最初のtargeted testは6成功。その後、最新callbackと古いlistener／再mountの追加testを全体gateへ含めた。
- 全体gate初回はcallback依存配列の警告で失敗したため、参照するhandlerを明示的に取り出して修正し再実行した。AppImage生成初回は新しいsnapshot名に対してcached xtaskの `CARGO_MANIFEST_DIR` が旧snapshotを指すことをbuild logで発見。誤ったsourceの生成結果を採用せず、対象cargo PID1280をTERMして中断（3分37秒）。補助scriptはsnapshot固有のxtask targetで再compileし、Tauri側cacheだけを共有する形に変更した。source修正後の実機成功は再生成・再配置後に確認する。

### 通知クリック修正版の検証と生成物（2026-09-06 01:06 JST）

- 最終 `cargo xtask desktop-ui-check` はlint、typecheck、143 file／1,109 test、Storybook build、browser test、visual操作smoke 14 testまで成功。Windowsでのpixel比較はskipであり、Linux baseline照合ではない。hookの追加3 testは、一覧更新後の一度だけの遷移、最新クリック・最新callback、破棄済みlistener／再mountを確認する。shell integrationの追加2 testはSettings／Control Centerを閉じた対象スレッド遷移と、無関係な通知の一括既読化0回を確認した。
- 今回の追加修正はfrontendのみ。Tauri／runtime sourceは前節で確認済みのものから変えておらず、既存のnative unit／e2e結果を再利用する。新しいAppImageのLinux compileは実行したが、それを通知クリックの実機成功の代替とはしない。
- 生成は `/home/kingyosun/kukuri-889-build.oUwluE` の分離コピー。snapshot専用xtask compile 2分45秒、Tauri release compile 3分28秒、Tauri build全体4分14秒で成功。build logがこのsourceを参照することと、変更したhook／DesktopShellPageのSHA-256がroot workspaceと一致することを確認した。
- 新成果物: `test-results/kukuri/issue-889-activation-fixed-package/kukuri_0.1.8_amd64.AppImage`、114,833,912 bytes。SHA-256 `71ca5d589ee6043bb361189b0c1465c59d54dfeee977fa9af1ca8ae2ea2c5564`。同じdirectoryに `.sig`、`appimage-artifacts.json`、`test-updater-key.pub` を保存した。本番鍵は使用せず、今回も新しい一時鍵なので旧成果物とのupdater更新pairには使わない。
- GIO検査 `local=1 tls=1` と公開鍵一致検査が成功。実成果物に対する `cargo test -p xtask signed_appimage_accepts_only_matching_bundle_and_key -- --ignored` は1成功（正常署名受理、改変・別鍵・署名欠落の拒否）。diff検査も成功。
- 01:06 JSTにUbuntuの旧実行をQuitで終了し、`2026-09-05T16:06:33.204316Z` の `desktop runtime shutdown completed`、window／tray消失、元端末のprompt復帰を確認した。同じ `/tmp/kukuri-889-smoke.SlPTEA` を指定する環境は保持。Windows `tauri:dev` は起動したままで、追加の返信・DMは送っていない。ユーザーによる新AppImage再配置後に、保持済み返信と修正後のクリック遷移を確認する。

### クリック修正版の再配置後（2026-09-06 01:13〜01:19 JST）

- ユーザーの配置・置換・実行権限付与後、Ubuntu端末でSHA-256 `71ca5d589ee6043bb361189b0c1465c59d54dfeee977fa9af1ca8ae2ea2c5564` を確認。前節のクリック修正版と一致した。
- 同じ `/tmp/kukuri-889-smoke.SlPTEA` を指定する起動履歴で実行し、再同意なしでメイン画面へ到達。元投稿22:04:23とWindowsからの返信00:35:23の2件、通知1件・未読1件を確認した。OS通知ON・本文preview OFF・通知サービス接続済みも保持されている。
- Ubuntuのendpoint IDは同じで待受portが変わっていた。Windows側PID74448の待受が継続していることを読取り専用で確認し、Ubuntuから前回確認済みのWindows LANチケットを1回追加した。Windows側は「接続中・Direct P2P」を表示したため、Windows側での追加チケット操作は行っていない。再起動後の手動接続情報の永続化成功を主張しない。
- アプリは両側とも起動中。Ubuntuは設定の接続画面、Windowsは検証スレッド上のコントロールセンターを表示。クリック修正を同じ非表示条件で検証するため、元投稿へ `Issue #889: 通知クリック修正後の再確認用返信です。` の公開返信1件を予定し、送信時承認を求める。この時点で追加返信の入力・送信はしていない。

### 通知クリックの実機再検証（2026-09-06 01:23〜01:29 JST）

- ユーザーの「OKです」という送信時承認後、Ubuntu側をSettingsの接続画面のままcloseで格納。今回のPIDは68935、初期化ログは `2026-09-05T16:14:40.910204Z`。Windows側で元投稿・公開scopeを確認し、`Issue #889: 通知クリック修正後の再確認用返信です。` を1件だけ送信した。返信の表示時刻は01:26:04。
- Ubuntu側で `Reply`、`Open kukuri to view this activity.` のOSバナーを実際に確認した。バナーを狙った最初のクリックではFilesが前面になり、kukuriは非表示のままだった。通知履歴には同じReplyが残っていたため、バナー消失とクリックの競合として扱い、クリック処理の失敗・再送の根拠にはしなかった。
- GNOME通知履歴から同じReply本文をクリックし、OS履歴を閉じた後、Settingsが自動で閉じてThread Columnが開いていることを確認。元投稿22:04:23と返信00:35:23／01:26:04が表示された。アプリ内の通知行やスレッドを先に開く操作はしていない。これで前節の一覧更新競合・Settings遮蔽の修正を実機で確認した。
- GNOME側の「kukuriの準備ができました」という通知は今回も出た。OS通知クリックによるwindow表示とアプリ内遷移は成功したが、OSのフォーカス制御や音の全条件を成功扱いしない。raw返信本文はOS通知に露出していない。今回までの公開投稿は元投稿1件と承認済み返信2件であり、他の送信・編集・削除はしていない。

### ファイル選択と最大化時の観測（2026-09-06 01:33〜01:39 JST）

- 空の投稿composerからnativeのファイル選択ダイアログを開いた。ファイルを選ばず「キャンセル」で戻り、添付なし・本文空欄を確認してcomposerを閉じた。ファイルの読込み・アップロード・投稿送信はしていない。ダイアログ表示と取消の確認であり、media読込み・codecの成功の代替ではない。
- ウィンドウ位置変更から最大化する操作の後、以前のwindow右辺／下辺に相当するL字状の帯と背景の残像が残った。通常サイズへ戻すと消え、再度最大化すると同様の境界が残ることを観測した。リサイズ中の旧座標へのクリックでプロフィール表示に移ったが、プロフィールの保存・変更はしていない。最終的に空composerは閉じている。
- 描画残像がアプリ／WebKitGTK／compositor／RDP／画面採取のどの層によるものかは未確定。一般的なGPU無効化を実装せず、利用者にも同じ残像が見えるか確認する。最大化状態の描画正常とはまだ判定しない。

### 本人による再起動後と外部リンクの観測（2026-09-06 01:49〜01:53 JST）

- ユーザーから、process終了・再起動後は残像が出なくなったとの報告を受けた。担当者も再起動後の通常windowと、その後1回の最大化でL字状の残像が再現しないことを確認した。描画コード・GPU設定は変更していない。前回からRDPの表示領域・windowサイズも変わっており、原因を確定した修正ではなく再起動後の非再現として記録する。
- 元投稿と返信2件は引き続き表示され、通知Columnは2件・未読0件になっていた。未読状態を変更する操作は今回担当者からは行っていない。
- リリース画面の「最新リリース」をクリックして外部リンクを確認。クリック後もアプリ画面がそのままで、ブラウザーwindowやエラー表示は観測できなかった。同じボタンを連打せず、既存sourceを確認した。
- `ReleasePanel.tsx` の資料リンクは通常の `a href`／`target='_blank'`、feedback経路は `window.open`。現行Tauri側に外部URLをOSへ渡すopener／new-window処理は見つからなかった。caller候補は同panelの外部送信先・資料・node開示・feedbackと `ReportRoutingDialog` のpolicy／権利侵害申出リンク。新しい実装方法・安全なURL境界・全caller・自動再現testはこれから確定する。ここまでで追加code変更・ブラウザーへの代替手動遷移は行っていない。

### 外部リンクの修正と境界検証（2026-09-06 02:05〜02:26 JST）

- AC-4・5／INVAR-2・3／INV-5／TR-5のExisting-gapとして、既存のReleasePanel（送信先、資料4件、node開示、feedback）とReportRoutingDialog（policy、権利侵害受付）を固定。`useExternalLinkOpener` → GUI専用 `commands::external_url::open_external_url` → Linux OpenURI portal／Windows ShellExecuteの順にする。通常browser anchor、内部skip link、blob downloadは変更対象から除外。callbackの自動起動・自動retry、通報や診断本文の付加、汎用shell／file／mailto許可は追加していない。
- frontend再現testは修正前に、最新リリースクリック後の `open_external_url` が期待1回に対して0回で失敗。修正後は成功。全release固定資料・feedbackと通報2導線、pending中の重複抑止、失敗後の明示retry、3localeのエラー表示、browserの非介入を追加testで確認。エラーにはnativeの生の文字列やURLを載せない。
- Tauriの既存startup／終了gateはそのまま適用し、sink側もReady／停止中を確認する。HTTP(S)の絶対URL・hostあり・credentialなしを検証してからOS adapterを呼ぶ。拒否URL18種以上・同意待ち／初期化中／停止中では偽OS sinkの呼出し0回、OS失敗からの明示retryでは計2回を確認した。CLI対応表は139入口となり、新規1件はGUI専用OSとして除外。最初の対応表testでは先頭にOS行を置いたことで既存negative fixtureの行順仮定を壊したため、既存先頭を維持する配置へ直し5 test成功。
- 実装選定はFirecrawlで取得した[Tauri公式opener資料](https://v2.tauri.app/plugin/opener/)、取得したopener 2.5.5／open 5.4.3のsource、[OpenURI仕様](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.OpenURI.html)とashpd 0.13.13のsourceを使用。Tauri openerの自動JS注入／plugin権限は導入せず、既存app gateを通す。Linuxはashpdのtokio／open_uri featureだけ、WindowsはopenのShellExecute featureだけを追加。両crateのlicenseはMIT（cargo metadata上）。bundle全体のlicense／codec確認をこれで代替しない。
- Linuxの隔離D-Bus試験初回はtest fixtureの型とmethod名の誤りで失敗。通常session設定にはservice自動起動があり、実portalが起動し得ることも確認したため、standard service directoryを含めない専用bus設定を追加した。そこでサービス不在・成功・取消・拒否・応答停止・回復を実OpenURI／Request.Response契約で検証し、5 test成功（0.24秒）。ホストのOS設定は変更していない。最初の隔離busが起動したportalのPID1343／1348／1363はtest後に存在しないことを確認。テスト中の詳細error出力は除去した。
- Windows Tauri lib 38 test、`cargo xtask tauri-check`、command parity 5 test、`cargo xtask e2e-smoke` 6 stepが成功。`cargo xtask desktop-ui-check` はlint／typecheck、145 file／1,119 test、Storybook build、browser 58 test、visual操作smoke 14 testまで成功。後から追加したbrowser anchorのpointer／Enter testも2件成功し、外部URLはroute fixtureで遮断した。Windowsのpixel比較はskipであり、Linux視覚baseline照合ではない。
- 修正版のAppImage生成とWindows実画面確認を継続中。Ubuntuの外部ブラウザー起動成功は再配置後まで未確認。

### 外部リンク対応版の実起動で見つかった回帰（2026-09-06 02:35〜02:39 JST）

- 外部リンク対応版は02:32:55に生成完了。`test-results/kukuri/issue-889-external-links-package/kukuri_0.1.8_amd64.AppImage`、115,464,696 bytes、SHA-256 `a54585c6c97db8efe3a1d0eb76ac076d3a4acd89b523a550ee5c5e7b38c50faa`。署名とGIO検査は成功した。生成sourceは `/home/kingyosun/kukuri-889-build.IVtO7O`、xtask compile 2分51秒、Tauri release compile 4分06秒、Tauri build全体4分55秒。試験用一時鍵であり公開配布用ではない。
- ユーザーの再配置後、Computer UseでUbuntu端末のsha256sumを実行し、同じhashを確認。旧processは本人により `2026-09-05T17:33:42.937472Z` に終了していた。同じ `/tmp/kukuri-889-smoke.SlPTEA` を指定する履歴で起動したが、PID81717は「起動状態を確認しています」のまま、worker81810に `Cannot start a runtime from within a runtime` が出た。外部リンク操作にはまだ到達していない。
- 02:37:59に起動元端末でSIGINTを送り、shutdown completed、window／tray消失とprompt復帰を確認。追加投稿、同意変更、profile初期化やkeyringの変更はしていない。
- 新しいashpdのtokio featureがzbusの同期APIのblock_on実装も切り替えることを依存sourceで確認。既存Secret Service clientとの互換性を、同じTauri feature集合のLinux再現testで検証する。以前のportal／URL単体testだけでは、この起動時の組合せを検出できなかった。

### 起動時の依存互換性修正（2026-09-06）

- `blocking_dbus_client_remains_usable_inside_async_startup` を先に追加。Tokio task内からzbusのblocking connection builderを呼び、実サービス・keyringへ接続せず、存在しない一意のsocketへの接続失敗を期待する。変更前は実機と同じnested runtime panicで失敗した（既存の外部リンク5 testは成功）。
- ashpdのfeatureを `tokio` から `async-io` へ変更し、既存Secret Service clientが使っていたzbusのbackendとの互換性を保つ。アカウント・鍵・DB・起動手順の変更やkeyringの無効化はしない。OpenURIの取消・拒否・応答停止も同じテスト条件を維持する。
- WSL Ubuntu 22.04の分離コピーでTauri lib全43 testが成功（compile 1分34秒、test 4.03秒）。上記再現testと実OpenURI／Request.Responseの隔離bus testを含む。Windowsの `cargo xtask tauri-check` と `cargo xtask e2e-smoke` も変更後に成功。frontend sourceは前節の全体gate成功後から変えていないため、その結果を再利用する。
- 互換性修正版は `/home/kingyosun/kukuri-889-build.cot7d4` で生成。Cargo.toml／Cargo.lock／external_url.rsのSHA-256がroot workspaceと一致することを確認した。xtask compileは2分49秒、Tauri release compileは3分27秒、Tauri build全体は4分16秒で成功。続いて同じsnapshotのLinux release条件でTauri lib全43 testが成功（compile 2分15秒、test 4.04秒）。GIO分離検査も `local=1 tls=1` で成功した。
- 02:53:29 JSTに新しい `test-results/kukuri/issue-889-external-links-compat-package/kukuri_0.1.8_amd64.AppImage` へ保存。115,018,232 bytes、SHA-256 `bc6d8ace008c0ef1bf7b36f9feca78f633932e8944c070c257f52aa9afd6e167`。manifestのhashとも一致し、同directoryに署名・manifest・検証用公開鍵を保存した。検証用一時鍵なので公開配布用ではなく、前の成果物とのupdater更新pairにも使わない。前節の起動失敗したAppImageは証跡として残し、修正版として再提示しない。実機は旧実行を終了したままで、ユーザーによる再配置後に起動と外部リンクを再検証する。
- 保存した実成果物に対する `signed_appimage_accepts_only_matching_bundle_and_key` も成功（1 test、10.29秒）。正常署名の受理と1 byte改変・別公開鍵・署名欠落の拒否を確認した。

### 互換性修正版のUbuntu実機確認（2026-09-06 02:57〜03:02 JST）

- ユーザーの再配置・実行権限付与後、Ubuntu端末でSHA-256 `bc6d8ace008c0ef1bf7b36f9feca78f633932e8944c070c257f52aa9afd6e167` を確認した。RDP経由の最初の文字入力は画面へ現れず、履歴操作も一度escape sequence表示になったため、未実行入力をCtrl+Cで取り消してから既存の確認・起動履歴を使用した。入力不明のまま実行や連打はしていない。
- 同じ `/tmp/kukuri-889-smoke.SlPTEA` を指定する既存履歴で起動し、02:59 JSTにメイン画面へ到達。前版の「起動状態を確認しています」で停止する問題は再現せず、起動元端末にもinitializedの行が現れた。本人への再同意要求・keyring無効化・アカウントや保存領域の初期化は行っていない。最大化後も前に観測したL字状残像は現れなかった。
- コントロールセンター → リリースと更新 → 資料の「最新リリース」を1回クリック。03:01 JSTにUbuntuのFirefoxが開き、アドレス欄の `github.com/KingYoSun/kukuri/releases/tag/v0.1.8-preview.1` と同名リリース本文を確認した。ブラウザーのアドレス手入力・代替URL遷移はしておらず、アプリのOpenURI経路による実起動の成功である。Firefoxの翻訳提案は操作せず、GitHubログイン・投稿・ダウンロードもしていない。
- kukuriへ戻り、保存済み通知2件・未読0件と、既読通知から元投稿22:04:23／返信00:35:23／01:26:04の3件が同じスレッドに表示されることを確認した。今回の新規投稿・返信・編集・削除は0件。OS通知設定はON・通知サービス接続済みの表示を確認しただけで変更していない。起動中のアプリは同スレッドを表示したまま残す。
- リリース画面のupdaterは失敗表示のままであり、今回の外部ブラウザー起動成功をupdater実更新成功とは扱わない。Windows側の外部URL完全照合、他のnative外部リンク導線、OS連携全体の未実施条件もこの1件から成功へ一般化しない。

### 追加OS連携とsingle instance（2026-09-06 03:08〜03:28 JST）

- 同じ互換性修正版 `bc6d8ace…afd6e167` と `/tmp/kukuri-889-smoke.SlPTEA` を使用。元投稿のリンクコピーからUbuntu Filesの未実行入力欄へ貼り付け、`#/timeline?topic=...` の値が別アプリで読めることを確認した。URL実行・送信はしていない。
- 診断表示のため開発者パネルを一時ONにし、Releaseの診断エクスポートを1回実行。ダウンロードdirectoryに新しい `kukuri-diagnostics.txt`（1.0 kB）が生成された。本文をUbuntu Text Editorで確認し、version 0.1.8、Linux x86_64、購読2件、未読0、通知サービスavailable、CN idle等とredaction表示を確認した。観測した出力に秘密鍵・auth token・DM本文は含まれていない。自動取得が失敗した理由は公開manifestにLinux platformがないこと。#890待ちの公開更新と、これから行う検証用更新を区別する。確認後に開発者パネルをOFFへ戻した。
- 03:25にwindow closeで非表示化。03:28にUbuntuの別端末タブから同じ一時profileを明示して同じAppImageを1回実行すると、保存済み3投稿のスレッド画面へ復帰した。実行前後の `ps -C kukuri-desktop-tauri -o pid=,comm=` はどちらもPID85905の1件のみ。2つ目の起動は終了し、元のprocessが維持された。これはURI引数なしのwarm single instance／hidden restoreの確認であり、cold deep linkや別profileへの転送の成功ではない。
- 今回の追加投稿・編集・削除・upload・OS permission変更は0件。診断fileはローカルに保存しただけで外部へ送信していない。

### SIGTERM後の再起動・保持（2026-09-06 03:31〜03:34 JST）

- PID85905の実行fileがAppImageのmount内 `usr/bin/kukuri-desktop-tauri` であることを確認してから、当該PIDだけへSIGTERMを送った。window・trayが消え、起動元端末に `2026-09-05T18:31:06.681823Z desktop runtime shutdown completed` とpromptが現れた。強制kill・profile削除はしていない。
- 同じ `/tmp/kukuri-889-smoke.SlPTEA` を指定する履歴で再起動。03:33にメイン画面と通知2件・未読0、03:34に既読通知から元投稿22:04:23／返信00:35:23／01:26:04の3件を確認した。再同意・別identity初期化の要求はなかった。Iroh全データやprivate capabilityの保持まで、この表示結果から推定しない。
- 再起動後はPID92234、initializedは `2026-09-05T18:32:59.296095Z`。更新試験一式の配置待ちにするためトレイのQuitで終了し、`2026-09-05T18:36:39.798669Z desktop runtime shutdown completed`、window／tray消失とprompt復帰を確認した。Ubuntuのアプリは終了した状態で残す。

### 依存noticeの別workspace欠落修正（2026-09-06）

- 既存 `generate-third-party-notices.ps1 -Check` は更新不足で失敗した。生成器はrootのCargo metadataだけを収集し、別workspaceのTauri依存を含めていなかった。単独の一時出力ではRust713／npm137件だったが、ashpd等のdesktop-only依存は欠落していた。
- 複数metadata fixtureのtestを先に追加し、旧generatorでは単一string引数として扱われるため失敗することを確認。`RustMetadataPath` を配列対応にし、通常実行ではrootと `apps/desktop/src-tauri/Cargo.toml` のlocked metadataを集計する。既存単一fixture入力・ordinal順・同一name/version/licenseの重複排除・workspace crate除外・UNKNOWN拒否を維持した。
- fixtureの旧契約、Tauri-only依存の収録、共有依存1行、入力順逆転でもCheck一致、metadata欠落拒否、desktop-only UNKNOWN拒否と失敗時既存出力不変のtestが成功。実lockfileで再生成したnoticeはRust1,075件・npm137件となり、実 `-Check` も成功した。これは全platformの依存一覧であり、AppImageに全件がlinkされるという意味ではない。今回以前の依存更新分も生成結果へ含まれる。
- 上記fixtureはPowerShell 7とWindows PowerShell 5.1の両方で成功した。`cargo xtask asset-check` はasset12件・file64件で成功。非コードasset自体は変更していない。
- AppDirの読取りではWebKitGTK／GTK／Ayatana AppIndicator／GStreamer core libraryの同梱を確認した。一方 `usr/share/doc` のcopyrightは17 directoryのみで、全libraryへの対応確認は未完了。Cargo一覧を同梱OS libraryのlicense照合や再配布条件の確認完了に置き換えない。
- 更新pair新版のAppDirで `ldd` の未解決依存は0。RUNPATHは `$ORIGIN/../lib`。ただしlibc／libgcc_s／libstdc++／libGL／libEGL／libgbm／libdrm／libX11等はbuild hostから解決されるため、AppImage単独で全依存を持つとは扱わない。GStreamer関連はcore／baseのlibrary10件を確認し、GStreamer plugin directoryは見つからなかった。再生可能codecは実機のホストplugin構成と再生試験で別途確認する。
- 同生成hostのpackage版: WebKitGTK／JavaScriptCoreGTK `2.50.4-0ubuntu0.22.04.1`、GTK `3.24.33-1ubuntu2.2`、Ayatana AppIndicator `0.5.90-7ubuntu2`、GLib `2.72.4-0ubuntu2.9`、GStreamer core `1.20.3-0ubuntu1.1`、base `1.20.1-1ubuntu0.6`。これはbuild hostのdpkg情報であり、Ubuntu Desktopの実ロード済みlibrary版の測定ではない。

### 検証専用の更新pair生成（2026-09-06 03:20〜03:37 JST）

- WSL Ubuntu 22.04の分離コピー `/home/kingyosun/kukuri-889-update-build.Ft14KM` で、同一source・同じ一時鍵を使った `0.1.8-test.889.1` → `0.1.8-test.889.2` を生成。Tauri configのversion・試験用公開鍵・endpointのみをコピー先で変更し、root workspaceの製品設定は変更していない。endpointは `http://127.0.0.1:18889/latest.json`、端末内HTTP許可はこの試験版だけ。署名検証は有効のまま。
- 旧版はTauri release compile 3分35秒／build全体4分25秒、新版はcompile 1分40秒／build全体2分28秒で成功。両方でxtaskの本体・署名照合とGIO `local=1 tls=1` が成功。旧新版とも114,981,368 bytes。
- 別鍵fixture作成では、CLIに空の `TAURI_SIGNING_PRIVATE_KEY` を渡すとfile指定より優先されて失敗した。初回helperはその時点でexit 1となり、一時秘密鍵はtrapで削除。正常pairは既に生成・検証済みなので再buildせず、環境変数をunsetした別の一時鍵で同じ新版のコピーだけを署名する処理へ修正した。この補助処理の失敗をpackage／署名の成功として隠していない。
- 同じbuild snapshotの `tests/updater_signature.rs` をLinux releaseで明示実行し、旧版・新版それぞれ1 test成功（0.66秒／0.70秒、初回compile 3分21秒）。正常署名と1 byte改変拒否を確認。続けて新版に別鍵fixtureを入力すると期待どおり `UnexpectedKeyId` で正常署名assertが失敗し、verifierの拒否を確認した。これは意図的なnegative入力であり、実機updaterの失敗表示の確認ではない。
- 成果物は `test-results/kukuri/issue-889-update-pair.qjR2MF/`。旧版SHA-256 `3d594f09865aaf1d4c420c1402deae08a356291a1449804e46f408682b8a22a9`、新版 `b8a2f01a81f41e99af6d7df6c874e9e2004c3555ca68a8e553aeaac0a76db9b3`。各manifestのhashと署名file本文が一致することも確認。秘密鍵は含めず、検証用公開鍵・正常署名・別鍵署名・READMEを保存した。
- 同directoryの `update-server.py` はPython 3標準libraryのみで、127.0.0.1へのbindと固定2pathだけを許す。normal／tampered／wrong-key／missing／interruptedの5 mode、hash不一致拒否、path traversal／directory一覧／余計なfileの拒否のself-testが成功（2.535秒）。実機でserverを起動したり、AppImageのdownload／install／restartを行ったりはまだしていない。
- 次はユーザーによるfolder一式のUbuntu配置と旧版への実行権限付与後、同じ検証profileで異常系→正常更新の順に確認する。以前の互換性修正版は残し、通常profileや公開Releaseは変更しない。置換不能やprivate capability等の未確認条件も維持する。
- 引渡しZIPは `test-results/kukuri/issue-889-update-pair.qjR2MF.zip`（229,979,044 bytes）。archive内の10 fileが予定したAppImage・署名・manifest・公開鍵・server・READMEだけであることを確認した。

### 更新pairのUbuntu配置確認（2026-09-06 05:19〜05:22 JST）

- ユーザーの配置完了後、Ubuntuのダウンロードdirectoryに展開済み `issue-889-update-pair.qjR2MF` と旧版への実行権限付与履歴を確認した。旧版・新版のSHA-256は前節と一致。`update-server.py` もローカル成果物と同じ `e1d05086d80f3c3290f0edeab829af8b0fcdd70b0a8629f09d59e8811cc40655` だった。
- Python `3.12.3`、`/tmp/kukuri-889-smoke.SlPTEA` の存在、旧版の実行可能属性、TCP18889にlisten中のprocessがないことを読取り操作で確認。OS設定や既存のAppImage・profileは変更していない。
- 新規配置ソフトの起動前にComputer Useの実行時確認を求める。検証用AppImageと端末内serverはまだ起動しておらず、download／install／restartも未実施。

### 更新異常系と正常取得のUbuntu実機確認（2026-09-06 05:23〜05:45 JST）

- ユーザーの起動時承認後、同梱serverのself-testがUbuntu Python 3.12.3で成功（2.527秒）。`127.0.0.1:18889` のtampered modeで起動し、同じ `/tmp/kukuri-889-smoke.SlPTEA` を明示して旧版を起動した。PID108093、initializedは `2026-09-05T20:27:03.509775Z`。元投稿・返信2件と通知2件／未読0を保持し、再同意を要求されずメイン画面へ到達。
- 版・状態・失敗理由を記録するため開発者表示を一時ON。Release表示で現在 `0.1.8-test.889.1`、候補 `0.1.8-test.889.2` を確認。manifest名や送信先説明は製品用固定表示のままだが、検証版の実endpointは端末内の限定server。ボタンの「インストール」はこの状態では `downloadUpdate()` のみで、適用・再起動は別操作であることをsourceでも確認した。
- tampered: 05:29に取得を1回実行すると `The signature verification failed` と日本語の署名検証失敗／インストールされない旨が表示された。再起動なし。server停止時の記録はmanifest1回・asset1回。直後の旧版SHA-256は `3d594f09865aaf1d4c420c1402deae08a356291a1449804e46f408682b8a22a9` のまま、processは同じPID108093の1件。
- wrong-key: server modeを変更し、05:33に「確認」でmanifestを再取得してから取得を1回実行。`The signature was created with a different key than the one provided` と同じ署名失敗表示。server記録はmanifest1回・asset1回。後続の旧版hash・PIDも同じで、適用・再起動なし。
- missing: 実fileを削除せずserverのasset URLを404にするmodeで、manifest再取得後05:37に取得を1回実行。`Download request failed with status: 404 Not Found` で失敗し、旧版表示を維持した。日本語は汎用の「更新サーバーに接続できませんでした」となり、実際のfile欠落と原因が合わない表示不足を観測。接続障害として成功扱いせず、後続の更新表示修正の検討対象へ記録する。serverの記録はmanifest1回（404応答はasset送信数には含めない）。
- interrupted: 05:41に転送途中で切断されるassetの取得を1回実行。`error decoding response body` と汎用の更新失敗表示となり、再起動へ進まなかった。server記録はmanifest1回・asset1回。missing／interrupted各試験の後にも旧版hashは `3d594f…b8a22a9`、processはPID108093の1件で維持されていた。
- normal: 05:44に正常modeを起動し、「確認」でmanifestを再取得。05:45の取得操作でダウンロードと署名検証が完了し、状態が「再起動待ち」、現在versionは `0.1.8-test.889.1` のままになった。「今すぐ再起動してインストール」は押していない。適用／再起動の実行時承認を求めて停止する。
- この時点で署名検証の無効化、公開Release変更、投稿・削除・OS permission変更はしていない。正常なdownload／verifyは実機確認済みだが、install／restartと置換不能、更新後の全データ保持は引き続き未確認。Ubuntuの旧版とloopbackの正常mode serverは起動中。開発者表示は検証のため一時ONのままで、試験終了時にOFFへ戻す。

### 正常更新の適用・再起動（2026-09-06 07:44〜07:52 JST）

- ユーザーの適用時承認後に再確認すると、現在版は `0.1.8-test.889.1` のまま、前回の「再起動待ち」から「更新あり」に戻っていた。serverは正常modeで継続稼働しており、同じ候補を再取得・署名検証してから適用した。待機中に再確認でstaged updateが置き換わる可能性を記録し、状態維持の自動testによる調査・修正は未実施。ユーザーが操作したことや定期処理だけが原因だったことは、この画面だけから断定しない。
- 07:45に「今すぐ再起動してインストール」を1回だけ実行。旧PID108093の `desktop runtime shutdown completed` は `2026-09-05T22:45:48.183669Z`、新PID128716のinitializedは `2026-09-05T22:45:48.912044Z`。停止完了後に新しいruntimeが起動したことを起動元端末で確認した。終了・再起動を手動で代替したり、ボタンを連打したりしていない。
- 自動再起動後、元投稿22:04:23／返信00:35:23・01:26:04の3件、通知2件／未読0を表示。再同意や別profileの初期設定は要求されなかった。Release画面でversion `0.1.8-test.889.2` と「最新です」を確認した。
- 07:52の読取り確認で、起動元の `kukuri_0.1.8-test.889.1_amd64.AppImage` のSHA-256が新版の `b8a2f01a81f41e99af6d7df6c874e9e2004c3555ca68a8e553aeaac0a76db9b3` へ変わり、配信用の `.2` fileとも一致した。ファイル名は `.1` のまま内容が新版へ置換されている。process一覧は新PID128716の1件のみ。
- 新PIDの実行fileはAppImageのmount内 `usr/bin/kukuri-desktop-tauri`。環境変数を必要な2項目だけに限定して読み、`APPIMAGE` は展開済み試験folderの `.1` file、`KUKURI_APP_DATA_DIR` は従来と同じ `/tmp/kukuri-889-smoke.SlPTEA` と確認した。環境変数一覧・秘密鍵・tokenを表示していない。
- 07:48に端末内serverをCtrl+Cで停止。正常modeの記録はmanifest6回／asset2回で、待機中の追加確認・最初のdownload・承認後の再download・更新後の確認を含む。端末内の更新試験であり、本番Releaseや本番署名の成功とは区別する。codec／license全体、置換不能、private capability・Iroh全state・keyring identityの完全な更新前後比較は未確認。
- 07:54に開発者表示をOFFへ戻し、設定を閉じた。更新後のアプリは通常の投稿表示で起動したまま残す。今回の追加code変更はなく、実機結果をこの記録へ反映した。

## 残作業

### 更新待機・欠落表示の修正と追加検証（2026-09-06）

- `ready_to_restart` からのcheck／重複downloadとasset 404の分類・3locale panel testを修正前に実行し、19件中9件が期待どおり失敗した。storeは定期／手動checkと重複downloadで検証済みobjectを保持し、Releaseの確認操作をdisableにする。asset downloadの404だけをmissingへ分類し、manifest 404や500、時間値の404msは元の分類を維持する。
- 修正後の対象20 tests／4 filesが成功。「あとで」→panel再表示→明示applyまで検証済みobjectを維持し、自動install／restartがないことも確認。
- 最初のdesktop-ui-checkは既存routes testの10秒timeoutで失敗（1127成功／1失敗）。同じsourceのroutes単独15件が成功し、timeoutやassertを緩めず全体を再実行。2回目は1128 tests／146 files、Storybook build、browser60件、visual smoke14件とlint／typecheckがすべて成功。Windowsのvisual smokeはLinux pixel baseline照合の代替ではない。
- その後の実Story描画で390px英語の再起動buttonが隣の確認buttonへ重なることを再現し、確認用scriptが `en/ready-to-restart: horizontal overflow` でexit 1となった。更新sectionの2つのaction rowのみを狭幅で縦並びにし、再起動文言を折返す修正を追加した。修正後の3回目desktop-ui-checkも1128 tests／146 files、Storybook build、browser60件、visual smoke14件、lint／typecheckが成功（Vitest 370.72秒）。
- 最終Story描画はen／ja／zh-CNの390px darkとjaの1280px light × 再起動案内あり／「あとで」操作後／asset欠落の計12条件が成功。確認disable・明示apply維持・欠落の案内・raw error非表示・horizontal overflowなし・page errorなし・focusとEnter操作を検証した。Storybook addonと同じaxe-core 4.13.0で更新sectionを検査し、各条件の違反0。自動検査だけでscreen reader適合やnative WebView成功とはしない。修正後の画像は `test-results/kukuri/issue-889-update-stories/` に保存し、日本語の長い再起動文言や中国語の欠落文言も視認確認した。
- 上記修正を含む新しい検証pairをsnapshot `/home/kingyosun/kukuri-889-update-build.b8Wmxb` から生成し、`test-results/kukuri/issue-889-update-pair.96sLkJ` へ保存した。旧版compile 5分12秒／build6分21秒、新版compile1分58秒／build2分52秒。両方のxtask署名照合・GIO isolationが成功。Tauriの実署名testも旧版1.07秒・新版0.77秒で各1成功（初回test compile3分41秒）。同梱依存collectorも175 ELF／116 packages、copyright欠落0で成功。
- 旧版SHA-256は `19167be8b99f9b3d9a861dca52e8008a1fd892cc30080321101f76ecc2def799`（114,993,656 bytes）、新版は `fa356149311ba95afbc72e4187b44943063858cf3a39d65b573a8f62989646d4`（114,989,560 bytes）。manifest本文のhash・署名file内容と一致。server自己testは2.620秒で成功。ZIPは229,221,090 bytes／SHA-256 `091a6edf3a3821fc064bae88daa196489b13c1a04758c4eaf3814456c840158b`、145 entriesで予定した本体・署名・metadata・README・server・著作権証跡だけを含む。秘密鍵なし。
- 13:05〜13:06 JST、ユーザーの配置完了後にComputer UseでUbuntuの `issue-889-update-pair.96sLkJ` を確認。両AppImageのhashは上記と一致、serverは既存の検証済み `e1d05086d80f3c3290f0edeab829af8b0fcdd70b0a8629f09d59e8811cc40655` と一致した。旧版の実行権限と `/tmp/kukuri-889-smoke.SlPTEA` の存在、port18889未使用を確認。従来アプリPID128716は1件のまま稼働している。新規AppImage／serverはまだ起動せず、Computer Useの実行時確認を求めた。

### 同梱OS依存の実測と再実行入口（2026-09-06）

- 署名確認済み `.2` AppImage（SHA-256 `b8a2f01a81f41e99af6d7df6c874e9e2004c3555ca68a8e553aeaac0a76db9b3`）を展開し、175 ELFを列挙。GNU build IDでUbuntu 22.04 build hostの116 dpkg packagesに対応付け、116すべてのcopyright原文を取得した。first-party実行file以外で未対応なのは `AppRun.wrapped` の1件。これはlicenseの全件確認・配布承認ではない。
- `scripts/release/appimage_runtime_inventory.py` に、AppDirを実行しない収集処理を追加。package版・source package／版、copyright原文、common-licenses、未特定ELF、非ELF、symlinkを保存する。出力先既存／AppDir内出力の拒否、build ID不一致、copyright欠落、portableなpackage path、tool失敗と未登録の区別等の9 testsはWindowsとWSL Ubuntu 22.04で成功。
- 実AppDirでの再実行も175 ELF／116 packages／copyright欠落0、未特定AppRun1で成功し、証跡を `test-results/kukuri/issue-889-runtime-evidence-v2/` に保存。CIは同じcollectorのtestと `runtime-evidence/` のartifact添付を行うよう変更した。GitHub上でのCI実行は未実施。xtaskは45成功／実署名fixture用1 ignoredで回帰成功。
- 同梱の主要packageはGTK 3.24.33、WebKitGTK／JavaScriptCore 2.50.4、GStreamer core 1.20.3／base 1.20.1。GStreamerの共有libraryは10件あるが、当該AppDirのELF一覧にはGStreamer codec plugin directory内のpluginはない。音声・動画再生やcodec同梱完了とは扱わず、host依存・欠落時の実挙動は追加検証する。
- `AppRun.wrapped` はTauri cacheの `AppRun-x86_64` とSHA-256 `f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f`、GNU build ID `84fe6e2b121f245ed5b8c05ccafe2dc1d45a9f37` が一致。[AppImageKitのAppRun source](https://raw.githubusercontent.com/AppImage/AppImageKit/master/src/AppRun.c) はSimon Peter／RazZzielのcopyrightとMIT形式の許諾を含むことを確認した。ただしcache binaryの対応source revision・外側AppImage runtime・静的依存・非ELF・source提供条件の確認は残るため、collectorは `redistribution_approved: false` を明示する。

### 修正版のUbuntu起動・欠落表示・再起動待ち（2026-09-06 14:32〜14:46 JST）

- ユーザーの起動時承認後、従来のPID128716をtray Quitで終了。`desktop runtime shutdown completed` は `2026-09-06T05:33:05.041419Z`。`/proc/128716` の消失も次の試験開始条件で確認した。停止済みの試験serverを探して強制終了したり、通常profileを変更したりしていない。
- 96sLkJ folderのserver自己testがUbuntuで2.525秒で成功し、14:35に127.0.0.1:18889のmissing modeを開始。別の端末tabで同じ `/tmp/kukuri-889-smoke.SlPTEA` を明示して今回の `.1` AppImageを起動した。PID184679、initializedは `2026-09-06T05:37:51.759975Z`。元投稿・返信2件、通知2件／未読0を保持し、再同意なしで起動した。既存のatk-bridge警告は14:38:16.366に再表示されたが、新しいGIO ABI警告はない。
- 開発者表示OFFのままReleaseの取得操作を14:39に1回実行すると、更新file欠落と未適用、時間を置いて「確認」からやり直す日本語案内を表示した。従来の「サーバーに接続できない」という誤った案内やraw HTTP errorは表示されない。適用・再起動なし。missing server停止時のrequest記録はmanifest1回。14:42のfile hash確認で旧版は `19167be8…2def799` のまま、processもPID184679の1件だった。
- 14:42〜14:43に正常modeへ切り替え、「確認」で失敗から更新候補ありへ回復。取得操作1回で14:44までにdownload／署名検証が完了し、再起動待ちの案内と適用button、disableされた確認buttonを表示した。長い日本語buttonは領域内で折り返され、隣のbuttonと重ならない。
- 14:44に「あとで」を選ぶと案内だけが消え、適用buttonは残った。disable状態の確認buttonをクリックしても候補再取得状態へ戻らず、適用待ちを維持した。設定を閉じて元の3投稿・通知2件を確認し、14:46にReleaseを開き直しても同じ適用buttonと確認disableを維持した。
- この時点でinstall／restartは未実行。30分の定期checkを跨ぐ実機観測もまだで、store単体の定期／手動共通入口testとは分ける。正常serverは起動中、アプリPID184679は同じ保存先で再起動待ち、開発者表示はOFFのまま。更新適用の実行時承認を求めて停止する。

### 更新表示修正版の適用・再起動確認（2026-09-06 14:51〜15:01 JST）

- ユーザーの更新適用時承認後、再表示したReleaseは検証済み更新の適用待ちを維持していた。14:52に残っている適用buttonを1回だけ押し、再取得や手動起動を挟まず自動で再起動した。
- 旧PID184679の停止完了ログは `2026-09-06T05:52:05.674211Z`、新PID186747のinitializedは `2026-09-06T05:52:07.817736Z`。旧runtime停止完了後に新版が起動した順序を確認した。
- 元投稿と返信2件、通知2件／未読0の表示を保持。14:56のアプリ情報でversion `0.1.8-test.889.2`、規約・privacy同意と年齢申告の従来日時を表示し、新しい同意操作を要求されていない。これらは読取りだけで、同意／年齢確認を代行していない。
- 14:59の照合で、起動元の96sLkJ folder内 `.1` AppImageのhashが新版と同じ `fa356149311ba95afbc72e4187b44943063858cf3a39d65b573a8f62989646d4` へ置換され、配信用 `.2` のhashとも一致した。processは新PID186747の1件。`APPIMAGE` は同folderの `.1` fileを指し、`KUKURI_APP_DATA_DIR` は同じ `/tmp/kukuri-889-smoke.SlPTEA` だった。環境変数はこの2項目だけを表示し、秘密値や全環境変数は取得・表示していない。
- 14:57に正常mode serverを停止。request記録は `manifest, asset, manifest` の3件で、確認→取得→新版起動後の確認に対応する。待機中の無効な手動確認やpanel再表示で追加取得は生じていない。今回は最初の30分定期checkより前に適用したため、30分跨ぎの実機確認とはしない。
- 新PIDでは既存のatk-bridge警告に加え、`iroh_docs::engine::state` の `sync state finish called but not in running state` を05:52:36.867377Zに1回観測した。停止／起動失敗や投稿消失は観測していないが、原因・差分起因性・全同期への影響はこの観測だけでは未確定。警告を抑制したり修正済みとしたりしていない。
- 15:01に設定を閉じ、更新後アプリを通常表示で残した。開発者表示はOFFのまま。今回の再起動と画面・保存先の保持を、未実施のprivate capability／Iroh全state／keyring identityの詳細比較や置換不能の成功で代用しない。Windows installerは未起動で、次の実行時確認が必要。

### Windows NSIS生成・署名回帰の結果（2026-09-06）

- 前節の分離snapshotで3回目の生成が完了。Windows release compile11分39秒、Tauri package全体14分13秒。NSIS本体とupdater `.sig` を生成した。実成果物に対する `published_bundle_accepts_only_its_valid_signature` をWindows releaseで明示実行し、正常署名の一致と1 byte改変拒否が1 test成功（0.20秒、test compile6分11秒）。
- 保存先は `test-results/kukuri/issue-889-windows-package.b3a1f57c43/`。`kukuri_0.1.8_x64-setup.exe` は25,802,216 bytes、SHA-256 `4e4080ecb5bd7e0124036923ffe9dc224df2675c2e89126746e564dd8354ab7a`。`.sig` は416 bytes、検証用公開鍵も保存した。生成した一時秘密鍵は削除済みであることを確認した。
- Authenticodeの状態は `NotSigned`。Tauri updater署名の成功と発行者証明書の署名を混同しない。まだinstallerを起動・インストールしておらず、Windows packaged UI／更新の実機成功は主張しない。通常の公開鍵・Windows config・既存dev process／profileを変更していない。

### Windows installerの実行開始と既存process確認（2026-09-06）

- ユーザーのインストール時承認後、NSISのSHA-256が上記の `4e4080ec…8354ab7a` と一致することを確認し、Computer Useで実installerを起動した。Welcome／Choose Install Locationへ正常に進み、この時点でSmartScreenや認証dialogは観測していない。
- 既存の配置と混ざらないよう、開始前に存在しないことを確認した `C:\Users\kgm11\AppData\Local\kukuri-issue-889-test` を指定。UIAの値設定APIがcache property errorとなったため、再観測で未変更を確認してから通常のclick／選択／文字入力で同じ欄へ指定した。インストール進行画面のOutput folderでもこのpathを確認した。
- Nextでインストール工程へ入ると、`kukuri is running! Click OK to kill it` の確認dialogが出た。対象マシンには既存のWindows dev版 `apps/desktop/src-tauri/target/debug/kukuri-desktop-tauri.exe` が起動中。強制終了のOKは押さず、未送信draft等を保存したうえでユーザーに正常終了を依頼した。Cancelも押していない。
- 現時点はinstallerの当該dialogで待機中であり、install完了・packaged app起動の成功ではない。既存dev process／通常profile、OS security／privacy設定は変更していない。Ubuntuの更新済み検証版と停止済みserverにも追加操作はしていない。

### ユーザーによるWindows install完了後の確認（2026-09-06）

- ユーザーからinstall完了・起動・通信許可を行ったとの報告を受けた。assistantは既存processの強制終了、OS通信許可の操作をしていない。installerは閉じており、CIM上はinstalled exeのprocess PID56228のみを確認した。以前のdev processは一覧になかった。
- 指定したinstall先には `kukuri-desktop-tauri.exe`（80,431,104 bytes）と `uninstall.exe` を確認。Computer Useが返した実行pathは `C:\Users\kgm11\AppData\Local\Packages\OpenAI.Codex_2p2nqsd0c76g0\LocalCache\Local\kukuri-issue-889-test\kukuri-desktop-tauri.exe` だった。このpathのfile存在とSHA-256 `4c39ce733472d65479bae12980b6e151711a0ae24494c01e44c50470b900ba89` を確認した。CIMのpath表記は `AppData\Local\kukuri-issue-889-test` 側であり、両APIの表記差を無視して架空のpathへ変更していない。
- Windows packaged画面をComputer Useで開き、通常画面・Control Center・アプリ情報の表示、version `0.1.8` を確認。規約・privacy／年齢申告の既存日時は2026-09-06 00:16:49と表示された。本人の手動操作の報告と、assistantの実画面確認を区別する。許可状態やネットワーク成功は通信許可だけでは断定しない。
- installed exeと現在のrelease directory内exeはhash・sizeが異なった。installed fileは13:08:12の80,431,104 bytes、後続の署名integration test後のrelease fileは13:14:53の71,780,352 bytesだったため、後続生成物を元のNSIS payloadと同一視しない。NSIS本体のhashとTauri署名検査成功、installed exeのhashはそれぞれ別の証跡として保存する。payloadをarchiveから再抽出したhash照合は未実施。
- 起動時の `KUKURI_APP_DATA_DIR` 指定を確認すると、ユーザー回答は「未指定」。現在の保存先で投稿・設定変更等の書込み試験は行わず、既存データを維持したまま、未存在確認済みの `C:\Users\kgm11\kukuri\test-results\kukuri\issue-889-windows-native-profile` を指定して本人に再起動を依頼する。起動コマンドは上記の実在確認済みinstalled exeを使い、OS全体の環境変数は変更しない。初回の年齢確認・同意が出た場合は本人が行う。

### Windows専用profileでの基本操作確認（2026-09-06）

- ユーザーが案内した専用保存先で起動し、年齢確認・同意を完了。`test-results/kukuri/issue-889-windows-native-profile/` にprofile metadata・lock・accounts・同意fileが作成されていることを確認。実行processはinstalled exeのPID23080の1件、ウィンドウID7145386。アプリ情報はversion0.1.8、同意・年齢申告日時は2026-09-06 15:32:25となり、以前の未指定起動の00:16:49とは区別できた。
- CodeGraphでsingle-instance pluginがprofile取得を含むapp setupより前に登録されることと、再表示callbackを確認してから実操作。閉じるbuttonでウィンドウが一覧から消えた後もPID23080は継続。installed exeをもう一度開くと元のウィンドウID7145386へ復帰し、processは同じPIDの1件だった。別runtimeの起動や、終了→新process再起動の試験とは区別する。
- この前後で `accounts.json` のSHA-256 `2805eb6823ba34b4ca373ac850f521a127cbab137785ce6542972337b4b125f8`、同意fileの `5f62243fb0d4f4194c188e70d4b6dac0038f8d8ab59682e4df18b3e2e559a9e7` は不変。秘密鍵・credentialは読んでいない。
- Releaseの公開リリースlinkを1回クリックすると、Chromeに `https://github.com/KingYoSun/kukuri/releases/latest` が渡り、Browserのtab情報で `https://github.com/KingYoSun/kukuri/releases/tag/v0.1.8-preview.1` と対応するRelease titleを確認した。後続の投稿・ログイン操作やReleaseの変更はしていない。元appへ戻った後に外部リンク失敗表示はなかった。
- Appearanceでdarkからlightへ切り替え、日本語の選択値が領域内に表示されることと、native selectにEnglish／日本語／简体中文の3項目が表示されることを確認。Escapeで言語を変更せず閉じ、darkへ戻した。OSの表示・security／privacy設定は変更していない。
- Profileの未保存欄に `kukuri-889-clipboard` を入力し、Ctrl+A／Ctrl+Cでcopy、別の未保存欄へCtrl+Vで同じ文字列が入ることを確認。画像file選択dialogを開いてEscapeでcancelし、file未選択を維持した。プロフィール保存は押さず、Resetで両欄を空へ戻して通常表示へ戻った。検証文字列はclipboardに残るが、profileとして送信・保存していない。
- Windows側は同じPIDで通常表示を維持。公開投稿・通知有効化・実終了後の再起動・Windows updater適用等はまだ未実施。保存・通信確認用の公開test投稿1件について、Computer Useの送信前確認を求める。

### Windows packaged投稿とUbuntu間の同期確認（2026-09-06 16:07〜16:16 JST）

- ユーザーが直前に承認した公開トピック `kukuri:topic:kukuri-889-smoke-20260905` と本文 `Issue #889: Windows packaged版の動作確認用投稿です。` を実画面で照合し、16:07:02に投稿buttonを1回だけ操作した。添付なし。投稿中表示が消え、入力欄が閉じて同じ本文の投稿1件が表示された。追加投稿・返信・撤回は行っていない。
- 送信直後は両端とも待機状態で、Windowsの検証トピックはピア0。Ubuntuでは同トピックの既存スレッド3件を表示していたがControl Centerのトピック検索には一致がなく、場所欄から同トピックのタイムラインを開き直した。接続詳細は4トピックを追跡し、検証トピックも待機・ピア0だった。この時点を端末間送信成功とは扱わない。検索で見つからなかった原因や再起動後の購読保持は未確定で、UI上の既存投稿保持とは分ける。
- Windowsの自分のチケットは仮想networkのIPを表示していた。実行中PID23080のIPv4 UDP bindが `0.0.0.0:64283`、以前の検証と同じLAN interfaceが有効であることを読取り確認し、公開endpoint IDを維持したLANチケットをUbuntuの「接続先を追加」へ16:13に1回入力・適用した。relay URL、seed、community-node認証・同意、OS firewall／privacy設定は変更していない。Windows側への逆方向チケット再入力も行っていない。
- Ubuntuの検証トピックが参加済み・ピア1へ変わり、新着1件を表示すると、Windowsの投稿本文と16:07:02の時刻が一致した。Windows側にも新着3件が現れ、表示後に元のAppImage投稿22:04:23と既存返信00:35:23／01:26:04の本文を確認した。双方の実画面で新旧の計4投稿を確認し、WindowsのControl Centerは「接続中 · Direct P2P」と表示した。これはLAN手動チケットの実通信確認であり、seeded DHT・Relay Supported P2P・Relay Fallbackの検証成功ではない。
- WindowsはPID23080の1件のまま、`accounts.json` と `kukuri.app-consent.json` のhashは前節の値から不変。秘密鍵やcredentialは読んでいない。開発者表示・OS通知設定は変更せず、両端を通常画面で残した。Ubuntuの検証serverも停止したままである。
- 次はWindowsの実終了→同じ専用保存先での再起動→投稿保持を確認する。Computer Useの起動APIは環境変数を指定できず、Windows端末UIでコマンドを代行することも禁止されているため、通常profileへの誤起動を避け、本人にtray Quitと前回の専用profile起動コマンドの再実行を依頼する。現在の成功を実終了後の保持、author／Iroh全stateの比較、Windows更新成功の代替にはしない。

### Windows packaged版の実再起動後の保持（2026-09-06 16:20以降 JST）

- 前節で依頼したtray Quitと専用profile起動後、ユーザーから起動完了の報告を受けた。CIMでは旧PID23080はなく、新PID70588の1件のみ。作成時刻は16:20:05.767074 JST、実行pathは前節と同じinstalled exe。Computer Useでも新ウィンドウID18091412を一意に選択し、単なる非表示からの再表示とは区別した。本人のQuit操作そのものやWindowsの停止完了ログはassistantが直接観測したものではない。
- 投稿操作や接続先の再入力をする前に、検証トピックの4投稿を確認。自分の16:07:02の投稿と既存22:04:23／00:35:23／01:26:04の本文が残り、自分の投稿には撤回actionが表示された。再送信・新規返信・撤回は実行していない。
- 専用保存先の `accounts.json` と `kukuri.app-consent.json` のSHA-256は、それぞれ `2805eb6823ba34b4ca373ac850f521a127cbab137785ce6542972337b4b125f8` と `5f62243fb0d4f4194c188e70d4b6dac0038f8d8ab59682e4df18b3e2e559a9e7` で、再起動前から不変。実画面のアプリ情報もversion0.1.8、規約・privacy／年齢申告日時15:32:25を保持した。秘密鍵・credentialは読んでおらず、これを全identity／Iroh state比較の完了とはしない。
- 再起動後のControl Centerは「待機中 · Direct P2P」。4投稿の保持は確認したが、手動チケットの再入力なしの自動再接続は成功と判定しない。今回の時点ではLANチケットの再入力やネットワーク設定変更も行っていない。
- Releaseの通知欄を読取り確認。利用可否の自動確認後は「許可済み」と表示されたが、アプリの「OS通知を有効にする」はOFF。ダイレクトメッセージ／メンションと返信はON、フォローとリポスト／静音モード／本文プレビューはOFFだった。通知の有効化・OS権限操作は代行せず、この画面を開いたまま本人に有効化を依頼し、同じ公開検証トピック内の16:07:02投稿への通知確認用返信1件について送信前確認を求める。返信は未入力・未送信。

### Windows通知確認の接続準備（2026-09-06 16:35〜16:39 JST）

- ユーザーの「OKです」で、前節のWindows投稿への通知確認用返信1件（本文 `Issue #889: Windows packaged版の通知確認用返信です。`）の送信許可を受けた。一方、実画面の通知masterは引き続きOFFであったため、返信は未入力・未送信のまま接続準備を行った。
- Ubuntuは検証トピックがdurable・ピア0、自分のチケットのportは33589を維持していた。Windowsは同じ公開endpoint IDのままport60948へ変わり、接続詳細の追跡対象がgeneral／dev／testの3件で、画面に保持された検証トピックは追跡されていなかった。表示データの復元と購読・到達先の復元は別に扱う。この時点では差分起因性や原因を確定していない。
- Ubuntuの実画面で再確認した公開endpoint IDとLAN到達先のチケットをWindowsへ1回入力・適用。続けて場所欄から同じ検証トピックを開き直すと、Control Centerで検証トピックが参加済み・ピア1、経路が「接続中 · Direct P2P」へ戻った。新しいトピックや投稿は作成していない。
- Windowsの通知画面へ戻し、master OFF、メンションと返信ON、静音／本文プレビューOFFを確認して残した。通知有効化は本人操作待ち。返信の送信許可を取り直さず、通知ONを確認した後に1回だけ送信して実機通知を確認する。

### Windows常駐中への通知確認用返信（2026-09-06 16:42〜16:46 JST）

- ユーザーから通知ONの報告を受け、Windowsの実画面でmaster ON、メンションと返信ON、静音／本文プレビューOFF、「許可済み」の表示を確認した。これらをassistantは変更していない。
- 設定を閉じ、ウィンドウの閉じるbuttonでWindows版を非表示にした。ウィンドウ一覧から消えた後もinstalled exeのPID70588は継続しており、Quitではなく常駐状態を試験前提とした。
- Ubuntuで同トピックが参加済み・ピア1であることを確認。16:07:02のWindows投稿から返信欄を開き、対象投稿・公開トピック・承認済み本文 `Issue #889: Windows packaged版の通知確認用返信です。` を照合して送信buttonを1回だけ操作した。16:44:06の返信が元投稿の子として表示され、入力欄も閉じた。添付なし、追加送信なし。
- 送信後のWindowsウィンドウ一覧と通知関連アプリの一覧には、操作対象となる通知バナーが現れなかった。RDP画面を最小化して再確認しても通知ウィンドウは返らなかった。これはOS通知が表示されなかった証明ではなく、Windowsへの返信到達・通知表示・通知クリック成功はいずれもこの時点で未確認。Windows版は非表示・同PID70588で残し、本人にWindows通知センターからkukuri通知の有無とクリックを確認してもらう。
- Ubuntuの該当スレッドは元投稿と新しい返信を表示したまま、RDPのみ最小化した。Windowsアプリを別手段で再表示したり通知クリックの代わりにアプリ内通知を開いたりして、OS通知クリック成功と混同する操作はしていない。通知・privacy／security設定変更、返信の撤回、追加返信はしていない。

### Windows通知未表示の切り分け（2026-09-06 16:48以降 JST）

- ユーザーがWindows通知センターに「通知なし」と報告。常駐PID70588の継続を確認してから、既存exeのsingle-instance再表示で同じウィンドウID18091412を開いた。アプリ内には16:44:06の該当返信が通知1件・未読1件として存在し、タイムラインにも新着1件があった。返信の通信・アプリ内通知生成は成功している。通知はクリックせず未読を維持し、追加返信は送っていない。
- CodeGraphで `background_notifications.rs::poll_once` と `os_notification.rs::show_platform_notification` を確認。Windowsはbundle identifierの `app.kukuri.desktop` をToastのAUMIDとして使う。Windowsのpermission commandは従来どおり固定 `granted` なので、画面の「許可済み」はOS設定の実照合を意味しない。OS設定変更や別アプリIDによる代用は行っていない。
- Codex側から通常のRoaming pathを読むと、15:18のmaster OFF設定と15:30の空cursorが返り、物理的なCodex LocalCache側と一致した。WSL経由の同じWindows volumeの読取りでは、実ユーザーのRoaming側の設定は16:41:08にmaster ON、cursorは今回の返信のreceived timestamp／IDまで進んでいた。これは保存先の仮想化による観測差であり、ユーザーのON操作が失敗した根拠とはしない。cursor更新はOSへの表示成功の証明でもない。
- 実ユーザーおよび共通のStart Menu Programsを読取りで確認するとkukuriのショートカットはなかった。一方、Codexの `LocalCache/Roaming/Microsoft/Windows/Start Menu/Programs/kukuri.lnk` は15:18:09に作成され、文字列には検証用exeと `app.kukuri.desktop` が含まれていた。生成済みNSISの `SetLnkAppUserModelId` macroも、このbundle IDをshortcutへ設定する実装だった。実ユーザーの `AppData/Local/kukuri-issue-889-test/kukuri-desktop-tauri.exe` は存在せず、現在のexeはCodex LocalCache配下だけにある。
- [Microsoftのdesktop toast手順](https://learn.microsoft.com/en-us/windows/win32/shell/enable-desktop-toast-with-appusermodelid)は、有効なAppUserModelID付きshortcutをStart menuに配置することを要求する。公式資料との照合で通知の前提となる登録不足を確認した。今回のCodex経由のinstaller起動による隔離配置が未表示の有力原因だが、正規のユーザー側install後の再検証と送信エラーの観測前に、他のOS要因まで否定しない。
- 同じNSISのhashは引き続き `4e4080ecb5bd7e0124036923ffe9dc224df2675c2e89126746e564dd8354ab7a`。製品コード・OS notification設定・registry・shortcutを直接変更せず、ユーザー自身がWindows Explorerから同じinstallerを起動して通常のユーザー環境へ配置する手順を依頼する。既存の専用profileと隔離配置を削除しない。再インストール後も自動起動を避け、同じ専用profileを明示して起動する。

### 通常ユーザー環境への再インストールと再接続確認（2026-09-06 16:58以降 JST）

- ユーザーがExplorerから再インストールし、同じ専用profileを指定して起動。新PID64592（16:58:13.258411 JST）の実行pathは通常の `C:\Users\kgm11\AppData\Local\kukuri-issue-889-test\kukuri-desktop-tauri.exe`。WSL経由で実volumeを確認し、同pathの存在とSHA-256 `4c39ce733472d65479bae12980b6e151711a0ae24494c01e44c50470b900ba89` の一致を確認した。旧LocalCache実行とは区別できる。
- 実ユーザーのStart Menu Programsに `kukuri.lnk` が16:58:01に作成され、通常の配置先と `app.kukuri.desktop` の文字列を含むことを確認。前節で欠けていたshortcut配置の前提は満たされた。これだけでOS通知表示・クリックの成功とはしない。
- 新ウィンドウID24184934で5投稿と16:44:06のアプリ内通知1件・未読1件を確認。専用profileのアカウント・同意fileのhashは前節までと同じ。通常Roaming側の通知設定はmaster／メンションと返信ON、quiet／preview OFFを保持し、dispatch cursorも前回返信の値だった。新しい返信は送っていない。
- 再起動後の追跡トピックは既定の3件だったため、検証トピックを再参加。WindowsにUbuntuの現在のLANチケット（port33589）、UbuntuにWindowsの現在のLANチケット（IPv4 UDP port51951）をそれぞれ1回入力した。Windowsの待受は `0.0.0.0:51951` と確認。今回のexeに対するWindows受信規則2件は有効・Allow・Private/Publicで、現行LANはPrivateだった。これらは読取りだけで、firewallやOS security設定は変更していない。
- 両側のリアルタイム接続はピア0のまま。開発者表示を一時ONにしてWindowsの診断を読むと、保存データの同期は1 peer経由で利用可能と表示される一方、最終エラーは `topic join pending: timed out waiting for initial topic join` だった。検証トピックを削除せずP2PスイッチだけOFF→ONに1回切り替えたが、recoveringを経て同じdurable／ピア0へ戻った。開発者表示はOFFへ戻した。リアルタイム再接続成功とは扱わず、原因・差分起因性は未確定のまま記録する。
- Ubuntuの稼働中AppImageログでは、前回返信16:44:06の直後に `connectivity_shape="live" direct_peer_count=1 docs_assist_peer_count=1` を確認できた。`sync state finish called but not in running state` は再接続の時刻付近にも出ていたが、今回のタイムアウトの原因とは断定しない。停止中の試験serverへの30分ごとのupdater接続失敗は既知のfixture停止に対応し、P2Pエラーと混同しない。
- Windowsは新PIDで通常画面・通知ON・未読1のまま。接続問題を未解決として残し、通常shortcut登録後のOS通知再確認に使う追加返信1件について実行時承認を求める。前回の返信許可を追加送信へ流用しない。

### 通常登録後の通知再確認用返信（2026-09-06 17:22〜17:30 JST）

- ユーザーの回答を提示済み返信1件の実行承認として受け、通常配置のWindows版PID64592と通知master／メンションと返信ON、quiet／preview OFFを再確認。Windowsを閉じるbuttonで非表示・常駐状態にした。
- Ubuntu側の元のWindows投稿（16:07:02）を返信先として明示し、本文 `Issue #889: 通常インストール後のWindows通知再確認用返信です。` を照合して1回だけ送信。17:28:27の返信が元投稿の子として表示され、入力欄が閉じた。添付・追加送信なし。途中でカラムメニューの表示遅延によりスレッド固定が意図せずONになったが、送信前に元の一時表示へ戻した。
- RDPを最小化してWindows側の通知ウィンドウを確認したが、ツールが返す一覧には通知バナーもアプリの復帰ウィンドウもなかった。通常Roamingのdispatch cursorは新しい返信IDとreceived timestamp `1788683307698` へ進んでおり、Windows側の通知処理が今回の返信を認識したことを確認した。cursorだけでOS表示成功とは扱わない。
- Windows版を非表示のまま維持し、本人に通知の表示有無とクリックを確認している。リアルタイム接続のtimeoutは未解決だが、今回の返信のWindows側認識は確認できた。OS通知の表示・activation成功と、通信経路全体の成功は別に判定する。
- その後、WSL経由で通常ユーザー側のWindows PowerShellを非対話実行し、WinRTの `CreateToastNotifier('app.kukuri.desktop').Setting` を読取り確認すると `Enabled` だった。通知送信・権限要求・設定変更は行っていない。同じAUMIDだけの `History.GetHistory(...)` を配列として数えると2件だった。OS側にkukuriの通知が保存されている証拠であり、バナーの目視やクリック成功とは区別する。履歴本文や他アプリの通知は取得していない。
- ユーザーが「表示され、クリックした」、続けてアプリ画面は「開かなかった」と回答。クリック報告後のウィンドウ一覧を2回確認してもkukuriはなく、PID64592は継続していた。通知表示は本人確認済みだが、クリック復帰は成功ではない。OS履歴には2件あり、クリックされた通知IDと今回返信IDの対応は取得できていないため、再起動前の通知との違いも含めて切り分けが残る。
- 基準commitと比較すると、WindowsのToast送信・`activate_main_window` の実装は変更されておらず、直接依存の `tauri-winrt-notification` 0.8.1も同じだった。周辺変更まで含む差分起因性は未確定で、Windows固有の既存問題を新しい必須条件へ自動追加しない。Windows通知クリック復帰の追加調査・必要な修正を今回の範囲に含めるか確認する。追加投稿、cursor削除による再送、別アプリIDでの通知送信は行っていない。

### Windows通知クリック復帰の実装・契約検証（2026-09-06）

- 追加承認後に方式を比較。NSIS版の既存経路は `ToastGeneric` とメモリ内 `Activated` callbackのみだった。[Microsoftのdesktop activationの選択肢](https://learn.microsoft.com/pl-pl/windows/apps/design/shell/tiles-and-notifications/toast-desktop-apps)では、COM activatorなしの非パッケージアプリはprotocol activationを使用する。既存のkukuri URI／single-instance経路を利用し、COMサーバーを追加しない方式を選択した。
- 修正前にhookへprotocol eventと初期URIのtestを追加し、8件中2件が失敗・既存6件が成功することを確認した（19.41秒）。通知バナー／通知センターの実クリック自体は前節の本人操作とウィンドウ観測を修正前証拠とし、API／mockの成功で置き換えない。
- Windows通知は自アプリAUMIDのまま、`activationType="protocol"` と通知IDだけのURIを持つXMLへ変更。XML DOMでtitle／bodyを組み立ててescapingとsilentを維持する。frontendは現在の通知一覧へのID照合を再利用し、strict URI validation、遅延一覧更新、listener cleanup、初期URIとlive eventの競合、session内の初期URI再処理防止を追加した。新しいlive clickは同じ通知URIでも処理する。
- Windowsのnative XML／URI tests3件とTauri lib全41件が成功。frontendの対象testsは31件成功し、ESLintとtypecheckも成功。無効URI・追加profile/body引数・別account/未知IDでnavigationしない条件も含む。通知の中身やOS設定を変更するための新APIは追加していない。
- NSIS hookは自アプリのStart menu shortcutにstub CLSIDを型付きGUIDとして設定する。専用fixtureはworkspace内の `.lnk` だけを作成・再読取りし、元のGUID不在、設定後のGUID一致、再適用、shortcutなし、不正file時の失敗を確認した。試作のSystem.dll構造体扱いでfixtureのaccess violationを検出したため、PROPVARIANTの明示サイズ確保と単一layout、native初期化関数を使用する形へ修正。修正後は複数の新規fixture directoryで成功した。通常Start menu／registryや稼働中アプリには触れていない。
- 再実行入口は `scripts/release/test-windows-notification-shortcut.ps1`。NSIS compilerがTauri cache以外にある場合は `-MakensisPath` で指定する。結果は `test-results/kukuri/notification-shortcut-contract.*` に保持する。
- third-party noticesの生成とCheckが成功し、e2e-smokeも6 step成功。全体desktop-ui-checkとtauri-check、分離snapshot `issue-889-windows-build.6f967cc780` でのNSIS生成・署名検査を開始した。実機は既存版PID64592を維持しており、修正版のインストール／通知クリックはまだ未実施。
- その後、tauri-checkは1分49秒で成功。desktop-ui-checkも1151 tests／147 files、Storybook、browser、visual smoke14件、lint／typecheckが成功した。Windowsのvisual smokeはLinux pixel baseline照合ではない。WSL Ubuntu 22.04の分離snapshot `kukuri-889-notification-check.8Vj2Ko` でもLinux release lib全43件が成功（compile1分52秒、tests4.02秒）。稼働中のUbuntu実機や同意・通知設定はこの検査で変更していない。
- 修正版NSIS生成と実成果物に対する `published_bundle_accepts_only_its_valid_signature` が成功（署名testのrelease compile5分08秒、test0.05秒）。成果物は `test-results/kukuri/issue-889-windows-package.6f967cc780/kukuri_0.1.8_x64-setup.exe`（25,799,306 bytes）、SHA-256は `7bce529cfa9334e206aa311028b99382c8358edc9db19587820479bc5e8e40ad`。同directoryに `.sig` と検証用公開鍵を保持する。分離snapshotだけの一時鍵によるTauri updater署名であり、Authenticodeは `NotSigned`、公開Releaseではない。生成用秘密鍵は補助scriptのfinallyで削除し、該当fileの不在を確認した。
- 次の実機工程は、本人が旧常駐版をQuitし、Windows Explorerからこの新しいinstallerを通常の `C:\Users\kgm11\AppData\Local\kukuri-issue-889-test` へ適用した後、既存専用profileを指定して起動する。旧形式の通知は比較対象から除外し、修正版から新しく発行された通知のバナー／通知センタークリックで同じprocessの再表示と正しい対象への移動を検証する。インストール／実クリック待ちであり、AC-WN-1の成功は未判定。

### Windows通知修正版の通常起動確認（2026-09-06 19:00以降 JST）

- 本人が新installer適用後の起動を報告。通常配置のexeから新PID11948（19:00:39）が起動し、ウィンドウID20972080で既存6投稿と未読通知2件の表示を確認した。インストール済みexeのSHA-256は `695332cdbc134966b2ebdb3b260b5f26245cac87d2a792c32a78d8eb1b35226a`。installer生成後に署名integration test用に再compileしたtargetのexeとは別に記録する。
- 通常Start menuの `kukuri.lnk` は19:00:27に更新された。Shellのmetadata読取りでAUMID `app.kukuri.desktop` を確認。ToastActivatorCLSIDの文字列getterは空を返したため、不在とは即断せず同fileのproperty storageを読取り確認し、AppUserModelのproperty ID26、型 `0x48`（CLSID）、設定したstub GUID `{5669FA60-8780-4392-8875-3CAB30E2F1F8}` のbytesを確認した。shortcut／registryの直接変更は行っていない。
- 設定画面でOS通知・DM・メンションと返信ON、フォローとリポスト・静音・本文プレビューOFFを目視確認。設定値は変更していない。検証トピックを再参加し、現在のLANチケットを両端末へ1回ずつ設定した。WindowsのIPv4待受portは63627、Ubuntuは33589。Windowsはdurable／ピア0、Ubuntuはエラー／durable／ピア0であり、リアルタイム接続の復帰成功とはしない。
- Windowsを閉じて非表示にし、同PID11948の常駐を確認した。既存通知は未読のまま、新しい返信はまだ送信していない。次は新しいバナー／通知センター通知で復帰と対象navigationを検証する。Computer Useの送信時確認に従い、実行するテスト返信をまとめて本人へ確認する。

### Windows通知クリック後の対象移動修正（2026-09-06 19:13以降 JST）

- 本人が提示した2件のテスト返信を承認した後、1件目の `Issue #889: Windows修正版の通知バナー確認用返信です。` だけをUbuntuからWindowsの16:07:02の元投稿へ送信。19:13:10の返信としてtimelineと同じthreadへの表示を確認し、RDPを最小化した。2件目の通知センター用返信はまだ送信していない。
- 本人が新しい通知をクリックし「アプリは開いたが通知対象へ移動しなかった」と報告。クリック後は同じPID11948・ウィンドウID20972080が再表示され、19:13:10の新規通知は未読のまま3件中の先頭にあった。元のtimelineには「新しい投稿を1件表示」が残り、thread columnは開かなかった。ウィンドウ復帰は成功したがAC-WN-1の対象navigationは失敗と判定する。
- Windowsの実 `Foundation::Uri::CreateUri(...).AbsoluteUri()` を使用するunit testを追加すると、生成した `kukuri://notification?id=...` が `kukuri://notification/?id=...` へ変わることを再現し、同一性assertionが失敗した。受信parserは前者のprefixだけを許可していたため、正規化済みURIのparser・live event・初期URIのtestsも3失敗／既存等35成功となった。クリック時の実argvそのものは採取していないが、OS正規化→parser拒否→navigation 0回を自動再現した。
- 生成URIにroot slashを明示し、受信はその正規形と従来のslashなし形式だけを許可する最小修正を行った。任意path、二重slash、異なるhost、追加profile引数、fragment、control文字を引き続き拒否する。修正後のWindows Tauri libは42件成功、対象frontend／shell testsは40件成功。全体gateと新しいNSISの生成・署名検証、本人の再install後の実クリックはこれから実施する。
- 切り分け中の読取りでは `kukuri:` 起動先が初回のCodex LocalCache側exeを指す結果も得た。同exeは旧版hashと一致し、旧版もdeep-link転送featureを持つ。これをURI正規化による拒否と混同せず、登録先の観測差・cold起動への影響は未確定として保持する。registry／OS設定の直接変更、URI本文のログ追加、新しい通知の無断再送は行っていない。
- このdeltaのtauri-checkは4.4秒、e2e-smokeは6 step／826msで成功。frontend全体gateと分離snapshot `issue-889-windows-build.fd34331b1f` でのNSIS再生成を開始した。Linux native通知・NSIS hook・依存は前回検証から変更しておらず、前回のLinux43件／shortcut契約の証跡を維持する。Windowsの実正規化を模した成功条件を実クリック成功の代替にはしない。
- desktop-ui-checkも全体成功。Vitest1160件／147 files（343.07秒）、Storybook生成、browser60件（23.1秒）、visual smoke14件（11.0秒）、lint／typecheckが通った。Windowsのvisual smokeはLinux pixel baseline比較ではない。新しいbuild snapshotと変更source／tests／既存NSIS hookのhash一致を確認し、このsnapshotのfrontend生成は56.73秒で成功した。
- 正規形対応版のNSIS生成と実Tauri updater署名testが成功。release compile4分40秒、package全体6分17秒、署名test用compile4分24秒、署名test1件0.05秒。新成果物は `test-results/kukuri/issue-889-windows-package.fd34331b1f/kukuri_0.1.8_x64-setup.exe`（25,796,066 bytes）、SHA-256は `d5a2f280d6afff35435c60d3ad3ad8a67f05b296459ae3f88d51b75a822c991c`。コピー先のhash一致と一時秘密鍵の削除を確認し、`.sig` と検証用公開鍵を同directoryに保持する。AuthenticodeはNotSigned、公開Releaseではない。旧成果物は比較証跡として保持し、新fileを本人の通常Explorer経由で再installしてから実クリックを再検証する。

### 正規形対応版の通知センター確認準備（2026-09-06 19:44以降 JST）

- 本人の再install後、新PID74944（19:44:51）・ウィンドウID136122780を通常の配置先で確認した。インストール済みexeのSHA-256は `0926c9fdf98dd018e849580b3410b49d77fd22fa52da378905f67431b0fc4854`。7投稿・未読通知3件を保持し、通常Roamingの通知設定もmaster／DM／メンションと返信ON、静音／本文プレビュー／フォローとリポストOFFだった。
- 検証トピックを再参加し、Windows側へ既存UbuntuのLANチケットを1回追加するとdurable／ピア0になった。WindowsのIPv4待受は52132。今回はUbuntuへのWindows側チケット再入力は行っていない。リアルタイム接続の成功とは区別する。
- 設定を閉じ、Windowsを非表示にして同PIDの常駐を確認した。承認済みの2件目 `Issue #889: Windows修正版の通知センター確認用返信です。` をUbuntuから元のWindows投稿へ1回だけ送信。19:50:27の返信表示と入力欄が閉じたことを確認し、RDPを最小化した。Windowsのdispatch cursorが今回の返信IDとreceived timestamp `1788691827733` へ進み、新しい返信の認識を確認した。通知設定変更・添付・追加送信なし。
- バナーはクリックせず、本人がWindows通知センターに残った新しい通知をクリックする工程を依頼した。Windows側のウィンドウを別手段で再表示せず、実クリックと対象navigationの結果を待つ。
- 本人が通知の存在と通知センターからのクリック、アプリ復帰、該当threadの表示を確認したと報告。その後の照合でも同じPID74944・ウィンドウID136122780を維持し、元投稿16:07:02と19:50:27の確認用返信を含むthread columnが7番目の選択中カラムとして新しく開いていた。notification／threadをアプリ内でクリックしてこの状態を作ったのではない。通知センターからの復帰・対象navigationはAC-WN-1の実機成功証跡とする。
- 通知一覧は4件／未読4件のままで、閲覧していない通知カラムを既読化したとは記録しない。OS通知click自体をmark-readの新しい契約にはしていない。正規形対応版のバナークリックはまだ再実施しておらず、前版のウィンドウ復帰だけの成功で対象navigationまで代用しない。

### 正規形対応版のバナー最終確認準備（2026-09-06 19:57 JST）

- 本人が最後の返信1件を承認。Windowsの開いていたthread columnを先に閉じ、6 columnsへ戻ったことを確認してからwindowを非表示にした。PID74944は継続した。同じthreadが既に開いている状態を、通知による新規navigation成功と取り違えないための準備であり、投稿自体は削除していない。
- Ubuntuから同じ元投稿へ `Issue #889: Windows通知バナーの最終確認用返信です。` を1回送信し、19:57:42の返信表示と入力欄が閉じたことを確認した。添付・追加送信なし。送信直後のRDP最小化操作はユーザー入力検出で実行されなかったため、再観測だけを行い、ユーザー側の操作を妨げる再activationは行っていない。
- 本人へ新しいバナーを表示中にクリックする工程を案内し、結果を待つ。通知センター経路の既存成功と、このバナー経路の未判定を区別する。
- 本人がバナーのクリックからアプリ復帰と該当threadの表示まで成功したと報告。後続の照合でも同PID74944・ウィンドウID136122780のまま、先に閉じたthread columnが7番目の選択中カラムとして再び開き、元投稿と19:57:42の確認用返信を含んでいた。通知センターに続き、バナーからの同process復帰・対象thread表示も成功として記録する。

### 通知元の個別投稿への自動スクロールの切り分け（2026-09-06）

- 本人が「該当投稿までスクロールしない」と追加報告。画面は正しいthreadの先頭側を表示し、通知元の返信は下方に存在していた。投稿／threadを追加クリックしたりスクロールで位置を合わせたりせず、観測状態を保持した。
- `handleOpenNotification` は `thread_root_object_id ?? object_id` を `openThread` へ渡すが、既存 `OpenThreadOptions.focusObjectId` を指定していない。`openThread` は対象投稿IDの指定がなければ `focusedObjectId` をnullにする。アプリ内通知でも同じhandlerを使う。基準commitとの照合でこの処理が変更されていないことを確認した。
- 既存 `reply notification click-through opens the source thread in timeline` は `threadId` までのURLとthread表示を検査し、投稿focusを要求しない。投稿リンク側には `focusObjectId` を持つ導線とscrollのtestsがあるが、通知とは接続されていない。Windows固有のactivation失敗や今回差分によるregressionとは区別する。
- 今回の固定条件に個別投稿のscroll位置は明示されていなかったため、共通通知導線の追加要件として扱い、OS通知／アプリ内通知から通知元の投稿まで移動する対応を#889へ追加するか本人へ確認する。製品コード・scope revisionはまだ変更していない。

### 通知元投稿へのスクロール実装・検証（2026-09-06、Scope revision v4）

- 追加承認後、PLANS／DESIGN／ADR 0014に従い、既存画面改善のbriefとAC-NS-1〜3／INVAR-NS-1を固定した。既存の投稿focus表示を使うため、新規token・カラム構造・design languageは追加しない。
- 修正前の新しいshell integration testsは6件中5失敗／欠落対象の安全なthread表示1成功。OS／アプリ内通知の両方で対象投稿がfocusされず、再クリックでも投稿bodyへのscrollがなかった。既存のthread scroll testはColumnCanvasの移動回数も数えていたため、今回は実際の投稿要素のfocusと、移動したscroll ownerが対象Threadのbodyであることを検査した。
- 共通通知handlerに `focusObjectId` を渡し、同topic／threadの追加pageから対象を取得するloaderを追加。欠落時は別投稿をfocusせず、cursor反復は打ち切り、取得失敗は既存のthread errorへ渡す。通知／threadの古い要求を取り消し、通常の非focus返信導線には新しいroute／Column取消条件を適用しない。
- session-onlyの `threadFocusRequestId` と選択中Columnに限定したfocus lifecycleを追加。対象投稿の準備後、所有するColumn bodyだけを即時に縦scrollし、focusはpreventScrollで渡す。通常refresh／追加pageで繰り返さず、同じ通知の新しい明示操作では再度移動する。OSのURI・通知本文・秘密値・既読API・永続schemaは変更していない。
- targeted handler／routing／loader30件が成功。追加pageを実際に分けたshell tests、欠落、private通知2経路も成功を確認。再クリック＋45行の追加pageを含む新testはCPU競合下で既定5秒を超えたため、assertionを維持し、この多段操作testだけ15秒の上限へ変更した。既存通知testのURL期待値は承認された投稿focus指定を含む形へ更新し、実投稿のfocus確認を追加した。
- Playwrightの新規4条件（1400px／390px、dark／light、通常／reduced motion、初期page／追加pageの対象）が11.9秒で成功。実際の投稿rectがColumn body内へ入り、別ColumnのscrollTopとdocument scrollを保持し、同じ通知の再クリックで再移動することを確認した。テスト初期化時にdocumentElementへ早すぎるアクセスをした試作は失敗として破棄し、既存theme storage keyを使って再検証した。
- 最初の全体gateは1173件中1171成功／2失敗。非active private ColumnのReplyから開くthreadのrouteが取消される回帰と、既存Bookmarks復元testの5秒timeoutを検出した。取消判定をfocus要求だけへ限定した後、private scope／Bookmarks復元／routing／通知focusの25件が45.13秒で成功した。現在は全体gateを再実行中であり、targeted成功だけで全体成功と扱わない。
- サイズ検査では、このIssueの前段でTauri依存のnotice対象を追加した生成済み `docs/THIRD_PARTY_NOTICES.md` が1276行となり新規超過を検出。生成正本を任意に分割せず、今回の依存網羅に必要な増加として同pathだけbaselineへ追加した。無関係なbaseline値は変更せず、再検査は成功。今回変更した手書きrouting／actionsは1000行未満を維持する。
- 実機用ビルドの最初の試行は、共有xtask cacheのrepo_rootが別snapshotを指す実ログを検出し中断した。Windowsは旧 `fd34331b1f`、Linuxは旧 `tTxLGS` のsourceを選んでいたため結果を採用しない。専用cacheに戻した次の試行も、上記private scopeの回帰判明時点で中断。遅いことを理由に止めたのではなく検証対象の誤り／既知の失敗が根拠である。Windowsの残った検証用一時秘密鍵2件は完全なpathを検証後に削除し、不在を確認した。Linuxの生成鍵も残存なしを確認。既存のinstalledアプリ・profile・比較用成果物は変更しない。
- 修正後のdesktop-ui-checkは全体成功。1175件／149 files（304.71秒）、Storybook、browser suite、visual smoke14件（7.6秒）、lint／typecheckが通った。多段の再クリックtest単独も9.43秒で成功し、設定した15秒内で全assertionを完了した。Windowsのvisual smokeはLinux pixel比較ではない。
- 未公開のbuildコピーを現在のsourceへ再同期して新しい生成前の固定点とし、Windows `issue-889-windows-build.0456cfe897`、Linux `kukuri-889-build.CnL5tr` の各専用xtask targetで生成を再開した。部分compile cacheは同じsource directory内でだけ再利用し、別snapshotの埋込みrepo_rootを再利用しない。
- 再開後は両方のpackage commandの実cwdが上記の正しいコピーであることを確認し、変更したsource／testsのhash一致を照合した。Windowsのfrontendは2705 modules／47.97秒、Linuxも2705 modules／1.36秒で成功。Windows release compile5分42秒・NSIS生成7分14秒、Linux release compile5分35秒・AppImage生成6分27秒が成功した。
- LinuxはAppImageのTauri updater署名照合に加え、lib全43件（tests4.03秒、test用compile3分32秒）とGIO isolation検査が成功。local file／同梱TLS利用と不整合module拒否を確認した。これらは新しいAppImageの実機スクロール確認の代替ではない。
- Windowsも実NSISに対する `published_bundle_accepts_only_its_valid_signature` が成功（test用compile5分19秒、test0.04秒）。成果物は `test-results/kukuri/issue-889-windows-package.6320c3a084/kukuri_0.1.8_x64-setup.exe`（25,781,189 bytes）、SHA-256 `d7020406551d1c2d87c9cb82dc7c70c7066af720b6465affab4d53f4360249b8`。同directoryに `.sig` と検証用公開鍵を保持する。AuthenticodeはNotSignedで、公開Releaseではない。
- Linux成果物は `test-results/kukuri/issue-889-notification-scroll-linux-package.kukuri-889-build.CnL5tr/kukuri_0.1.8_amd64.AppImage`（115,001,848 bytes）、SHA-256 `68dd6ae6bb7dd06fbd9b97ce42bcbd23d307f78a92618a2f622e14aef3cbcab9`。署名・manifest・検証用公開鍵を同directoryに保持し、manifestと実fileのhash一致を確認した。両OSの正常終了後の生成用秘密鍵の不在も確認した。旧installed版・既存profile・比較用成果物は保持し、本人の差替え後にOS通知／アプリ内通知の実機scrollを確認する。

### v4 Windowsアプリ内通知の実機スクロール確認（2026-09-06 23:30以降 JST）

- 本人の差替え・起動報告後、通常配置の新PID55016（23:30:46）・ウィンドウID47842946を確認。exeのSHA-256は `38fa58928e45f3c0e7459374fca7f634a22ff6df69889281216255e1a1160d21`。既存投稿と通知6件を保持し、開始時のThreadは先頭側を表示していた。
- Control Centerから通知一覧を開くと、検証トピックscopeの通知Columnが追加され、既存の一覧閲覧時の既読処理で6件の未読が0件になった。今回新たな既読API呼出しや既読契約を追加した結果ではない。投稿・通知の削除や新規返信は行っていない。
- 既存の19:57:42の通知をクリックし、同じ元投稿のThread内で該当返信が表示範囲へスクロールされ、既存のfocus枠で強調されることを画面で確認した。隣のTimelineは20:00:55の投稿が先頭の位置を保持した。
- Threadだけを手動で上方へスクロールして該当返信を表示範囲外にし、5秒待機（通常3秒の更新周期をまたぐ）後も位置が戻らないことを確認。同じ19:57:42の通知を再クリックすると、該当返信へ再び移動し強調表示された。初回移動・再クリック・通常更新時の位置保持はWindows installed版の実機成功証跡とする。
- OSの同一AUMIDの通知履歴を読取り確認すると1件、protocol activationの通知は0件だった。残る旧形式通知をv4 OS経路の成功証拠へ流用せず、新しい通知1件でOSからの復帰・個別投稿scrollを確認する。本文やURIを出力せず件数だけを確認し、通知送信・OS設定変更は行っていない。Linux側のv4実機確認も引き続き未実施。

### v4 Windows OS通知からの個別投稿スクロール確認（2026-09-06 23:48 JST）

- 本人の返信1件の承認後、PID55016とOS通知／メンションと返信ON・静音／本文プレビューOFFを確認し、Windowsを非表示の常駐状態にした。Ubuntuでは元投稿16:07:02を再確認し、`Issue #889: 通知から投稿へのスクロール最終確認用返信です。` を1回だけ送信。23:48:09の投稿表示と入力欄が閉じたことを確認した。添付・追加投稿・接続設定の変更なし。
- 送信後の観測時点でWindowsが再表示されていたため、こちらから再activationやアプリ内通知のクリックは行わず、既存ウィンドウの状態を取得した。同じPID55016・ウィンドウID47842946のThreadに23:48:09の返信が表示範囲内で強調されていた。隣のTimelineには新着1件の案内が残り、新しい投稿を表示する操作で結果を代用していない。通知は7件／未読1件だった。
- この再表示とスクロールが新しいOS通知の本人クリックによる結果であることを本人へ確認する。UI上の結果は観測できたが、クリック手段の確認前にOS経路の最終成功とは判定しない。
- 本人が「あっています」と回答し、新しいOS通知をクリックした結果であることを確認した。Windows v4のOS通知からの同process復帰・該当Thread選択・通知元の23:48:09の投稿へのスクロール／強調表示を実機成功と判定する。アプリ内通知の既存成功と合わせてWindowsの2入口を確認した。Linuxのv4実機やIssue全体の残条件・独立監査を完了したとは扱わない。

### v4 Linuxアプリ内通知の実機スクロール確認（2026-09-07 00:14〜00:16 JST）

- 本人が新AppImageの起動を報告。Ubuntuの画面上の起動コマンドで、新しい `issue-889-notification-scroll` directoryと従来の `KUKURI_APP_DATA_DIR=/tmp/kukuri-889-smoke.SlPTEA` を確認した。起動ログは2026-09-06 14:57:36 UTC、表示されたprocess IDは260487。配布元の成果物hashは前節のとおりで、Ubuntuへコピーした実fileのhashはこの工程では再取得していない。既存の元投稿・返信・通知2件が保持されていた。
- 検証準備のウィンドウ高さ変更で一部領域の描画欠けを観測した。最大化後に通常サイズへ戻し、表示が正常に戻ったことを確認してから通知操作を開始した。過去にも描画残像を観測しており、この操作だけで原因やv4による回帰とは断定しない。アプリ再起動・投稿変更・設定変更はしていない。
- 高さを制限した通常ウィンドウで、1:26:04の既存通知 `Issue #889: 通知クリック修正後の再確認用返信です。` をクリック。22:04:23のUbuntu元投稿のThreadへ移り、末尾の1:26:04の返信全体が表示範囲内へスクロールされ、focus枠で強調された。隣のTimelineは23:48:09の投稿を先頭に保持した。
- Threadの縦スクロールバーを手動で先頭へ戻すと、元投稿と0:35:23の返信が表示され、1:26:04の対象返信は下方の表示範囲外になった。同じ通知を再クリックすると、対象返信が再び表示範囲内へ移動し強調された。初回移動と再クリックはLinux AppImageの実機成功証跡とする（AC-NS-1／3）。
- もう一度Threadを先頭へ戻して通知一覧の更新ボタンをクリックし、5.2秒待機して通常3秒の更新周期もまたいだ。元投稿を先頭とする位置は維持され、対象返信への強制的な再移動はなかった。通知は2件／未読0件、隣のTimeline位置も維持された。新しい投稿・添付・通知設定変更は行っていない。
- Linuxのv4 OS通知からの復帰・個別返信への移動は未実施。WindowsからUbuntuの元投稿へ新しいテスト返信を1件送信する必要があり、Computer Useの送信時確認に従って本人の承認を求める。既存のアプリ内通知成功や前版のOS通知成功で代用しない。

### v4 Linux OS通知からの個別投稿スクロール確認（2026-09-07 00:22〜00:24 JST）

- 本人がテスト返信1件を承認。Linux側で通知サービス接続済み、OS通知／DM／メンションと返信ON、フォローとリポスト／静音／本文プレビューOFFを画面で確認し、設定は変更せず閉じた。Threadを元投稿22:04:23の先頭位置にしたままウィンドウを閉じ、非表示の常駐状態にした。
- WindowsでUbuntuの元投稿 `Issue #889: AppImageの動作確認用投稿です。` の返信欄を開き、承認された `Issue #889: Linux通知から投稿へのスクロール確認用返信です。` を1回だけ送信した。0:22:47の返信がTimelineと同じThreadへ追加され、入力欄が閉じたことを確認。最初の画面外の返信ボタン指定はbounds errorとなったため再観測し、元投稿までスクロールしてから操作した。追加投稿・添付はない。
- Linuxで新しいOSバナー `Reply / Open kukuri to view this activity.` を観測した。バナーへのクリック試行時には表示が消えており、背後のFilesの列見出しへ入力されたため、バナー経路のクリック成功とはしない。GNOME通知センターを開き、残っていた同じ新規Reply通知をクリックした。
- 通知クリック後、GNOMEの「kukuriの準備ができました」が表示され、非表示だったアプリのウィンドウが再表示された。通知センターを閉じる前から、右のThreadで0:22:47の対象返信全体が表示範囲内へ移り、既存focus枠で強調されていることを確認した。アプリ内通知・投稿・trayによる再表示や手動の位置合わせは行っていない。その後、時刻表示をクリックして通知センターだけを閉じ、同じ結果を確認した（AC-NS-1）。
- 通知一覧は3件／未読1件で、新規通知は未読のまま。隣のTimelineは23:48:09の投稿を先頭に保ち、「新しい投稿を1件表示」が残っていた。通知クリックを既読処理やTimeline更新の契約へ拡張していない。Linux v4のOS通知センター経路とアプリ内経路の両方を成功と判定する。OSバナーのクリック成功や、未確認の他環境・Issue全体の完了は主張しない。

### 旧範囲の残条件と生成時の履歴（Superseded: v5）

2026-09-06のユーザー回答「このWindowsマシンだけ」「環境が用意できないものは今検証しなくても良い」「XWayLandも今やる必要性は薄い」により、追加環境・XWaylandの今回の実行必須条件を延期へ変更した（Scope revision v2）。元の環境表は削除せず、未実施のUbuntu 22.04／Debian 12やGPU構成を成功と扱わない。現在のUbuntu 24.04とWindowsで可能な確認を継続する。

Windows生成時の補助失敗履歴: 分離snapshot `test-results/kukuri/issue-889-windows-build.91d4d0728b` の最初の補助実行はWindows PowerShell 5.1がsignerのpasswordなし警告を終端errorとして扱って停止したため、公開鍵生成の戻り値で成否を判定するよう補助scriptを修正した。2回目はsnapshotが元repo配下にあるためTauri packageが外側workspaceを誤認し、cargo metadataで停止。コピー先のTauri manifestだけに空のworkspaceを置き、元のstandalone条件を明示した。rootのmanifest・公開鍵・config・既存profileは変更していない。3回目の成功結果は前節に記録。Windows／Linux snapshotとrootのTauri Cargo.lockのhash一致も確認した。

T1の残る詳細棚卸し、T2のCI実行・残るbundle依存／codec／license確認、T3の終了・復元・切替競合、T4の残るOS連携、T5の置換不能・更新後全データ比較・定期待機確認、T6のWindows packaged操作等の延期外実機条件、T7の残る検証、T8の独立監査が残る。更新待機操作と404表示の修正・修正版の実適用、Windows NSIS生成とTauri署名検査は上記まで完了。未実施の実機・CI・署名付き公開配布・独立監査の代替にはしない。

### 現在の残る条件（v5）

| 対応 | 残作業と方法 | 終了条件 |
| --- | --- | --- |
| T1／T6 | 今回変更した入口・影響先・sensitive sinkと既存tests／実機記録を照合。不足だけを以下へ割り当てる | AC／INVAR／INV／TRとの対応が明確で、未分類を残さない。未変更のOS機能全体へ拡大しない |
| T3 | 起動／復元／切替と終了の競合、tray喪失、identity保護の未充足な証拠を制御した自動testsで補完 | 禁止mutation・終了後runtime再生成・誤identity生成・二重所有を防ぐ。正常起動／終了／保持の実機成功は再利用 |
| T5 | 置換不能時の実filesystem／updater経路、制御時計による定期確認、fixtureの更新前後保存状態比較 | 旧版・対象データの保持、失敗時の再起動0回、待機objectの保持。30分の手動待機や全製品データの手作業生成は不要 |
| T2／T4／T7 | AppRun／外側runtime等の依存・配布条件、登録先や起動引数等の変更影響、未実行のpath別検証・手順の同期 | 実成果物・OS API／process・隔離D-Bus・DOM／browser等の適切な証拠が揃う。実機で成功済みの通知等を再実施しない |
| T2／T8 | GitHub package CI・必須CI、固定PR headの独立監査、承認後の統合 | 必須CI成功・監査PASS・blocker 0。コミット／PR／マージは未依頼、#890の公開統合は対象外 |

描画欠け、atk-bridge警告、リアルタイム接続の未復帰は観測記録を維持する。今回の固定条件違反または差分による回帰の具体的根拠なしに、すべてを必須修正へ昇格させない。自動検証で判定できない具体的な問題だけが追加手動実機の候補となる。

この改訂では計画・ADR・runbook・本記録だけを同期した。製品code、OS設定、投稿、成果物、起動中アプリを変更せず、新たな実機検証・build・CIは実行していない。未実施条件をPASSへ変更していない。

### v5の自動検証補完とPR準備（2026-09-07）

- 本人がClose判断までの続行とコミット・PR・CI成功後マージを明示承認。追加手動実機を前提にせず、前段の独立確認で不足した証拠を限定して補完した。
- `DesktopShellPage.updateSchedule.test.tsx`は実shellの30分intervalと実update storeを接続し、制御時計で2周期を跨いでも同じ検証済みobjectを保持、追加download／install／restartなし、unmount後のcheck停止、明示apply成功を確認した。無関係な3秒pollだけを分離した。試作は未mockのnative eventと余分な通知設定IPC、timerの型で失敗し、対象境界を分離して修正した。製品timer／storeは変更していない。
- 独立確認担当が`existing_keyring`障害・回復のcharacterizationを追加。keyring get失敗／サービス不在／entry欠落でmarker・entry不変、file fallback生成0、回復後pubkey一致を確認。identity関連12 testsがWindowsで成功した。
- 同担当が終了待機中に公開された実hostを停止するtestを追加。実SQLite／Irohを再openして投稿・author・endpoint保持を確認。Windowsの対象1件（1.01秒）とlifecycle全5件（0.93秒）が成功。監査担当自身によるtest追加のため、最終PR headでは別担当による差分確認を行う。identityは必要な50行のtest追加で1004行となり、同pathだけoversized baselineへ追加した。無関係なbaseline値は維持する。
- `updater_install.rs`はTauriの実updater pluginに、loopback限定HTTPで実署名付きAppImageを渡す。mockはGUI runtimeのみで、download／verifier／installとfilesystem権限は実物。書込み不能な専用directoryでinstallが失敗し、既存fileのbytes・実行権限と隣のprofile sentinelが不変、権限を戻した明示retryで正常置換することを確認。Linuxの非root userで1件成功（test 1.27秒／release compile 6分55秒）。既存fileは実行しないsentinelであり、このtest単独で旧版アプリの起動や全profile内容を検証したとはしない。正常旧新版更新の既存実機証跡と実host保持testを併用する。生成済み署名fixtureを使い、端末への再配置や起動はしていない。
- 上記実updater testをLinux package CIへ追加。Tauriのmock runtime featureとtempfileはdev-dependencyだけに追加し、本番の署名・endpoint・keyring・保存形式は変更していない。
- 同梱物の独立照合でschema／xdg-mimeのcopyright収集不足を検出。追加2 testsが失敗した後、内容hashがbuild hostと一致する場合だけownerを対応付ける最小補完を行い、Windows／WSLの全11 testsが成功。既存AppDirの再収集は175 ELF／121 packages／copyright欠落0。非ELF 67・symlink33の分類、AppRunと外側runtimeのstatic依存、#890公開時のsource／notice提供条件は[同梱物の証跡](../runbooks/linux-appimage-runtime-evidence.md)に集約した。`redistribution_approved: false`を維持する。
- 現時点はPR準備。差分全体の必須CIと固定headの独立監査が残り、前段の「具体的blocker未発見」や対象test成功だけでClose可能とは判断しない。

### PR #903の独立監査で見つかった通知競合（2026-09-07）

- `93a3dee7`でPRを作成。package監査がCIのfixture相対pathとCargo testのcwdの不一致を指摘し、`10d6a09b`で`$PWD`起点の絶対pathへ修正した。旧headの重複CIは対象更新と既知のfixture不備を理由に停止し、最新headへ集約した。
- native担当はINV-3／4／5の12群（適合11、変更影響なし・追加実機対象外1、不適合0、未分類0）、package担当はINV-1／2／6の3群（適合3、不適合0、未分類0）でコード監査PASS。package担当はcollector11件、notice fixture、package5件、実署名1件、実Linux updater install1件を独立実行して成功した。CI成功やPR全体PASSとは別に扱う。
- frontend担当がAC-NS-3／AC-WN-2の段階を跨ぐ競合を検出。通知Aのtopic読取り完了→AのThread読取り待機→通知Bのtopic読取り待機→A完了→B完了の順序で、Aが先にhashを変更しBが破棄される。実shell統合testは期待`focus-reply-2`に対し実際`focus-reply-40`となり失敗した。既存の同一段階同士の競合testsだけでは検出できていなかった。
- `OpenThreadOptions.isCurrent`という任意のsession内callbackで、通知操作の世代判定を共通handlerからrouting／既存focus loaderまで引き継ぐ最小修正を行った。callbackを渡さない既存導線の挙動、OS引数、永続state、外部送信、Column構造は変更しない。追加手動実機・テスト投稿は行っていない。
- 修正後の通知focus／actions／routing／loader／実schedulerの5 files・40 testsが成功（41.07秒）。独立担当が追加したkeyring／実hostの2 characterization testsも別frontend監査担当が読み直し、有効な実証であることを確認した。最終headでのfrontend delta監査と必須CI成功を待つ。

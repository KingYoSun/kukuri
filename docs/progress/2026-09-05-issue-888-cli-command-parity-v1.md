# Issue #888: CLIコマンド対応と画面なしE2E

## 現在の判定

- 判定: Audit pending。#888本体の実装・ローカル検証を完了し、PR headの独立監査とCIを待つ。共有処理抽出の先行PR #899をマージ済み。本体のmerge／Closeはまだ行っていない。
- リスク区分: C
- Scope revision: `2026-09-05-issue-888-single-request-execution`（2026-09-05のユーザー決定により改訂）
- 基準commit: `6f89fae049170f5b77aa6ffb95da052f3fb05dfe`
- 仕様: [ADR 0049](../adr/0049-linux-gui-cli-control-plane.md)
- 完了条件: AC-1 全登録点の一意分類、AC-2 全対象操作とschema、AC-3 複数CLIの主要機能E2E、AC-4 再起動とCommunity Node有無、AC-5 秘密情報と認可境界、AC-6 1入力1回の実行・別入力の独立性・復元後の保護期間なし。
- 維持条件: INVAR-1 GUI・DTO・wire・署名、INVAR-2 private audience・epoch・秘密情報、INVAR-3 同意・認証・P2P経路、INVAR-4 一時profile限定の検証。

## 2026-09-05の責務見直し

ユーザー決定により、復元をまたぐ再実行だけでなく、意図的な別入力による重複の防止もCLIの責務から外す。1回の入力から処理を重複起動しない責務は維持する。

- 決定済み: CLIの操作キー必須化、入力間の重複排除、永続台帳、台帳のbackup／restore、復元後の5分間の保護を撤去する。同内容の別入力はそれぞれ既存domainの規則で処理し、CLIで意図を推定しない。
- 維持するもの: 認可・同意・audience、変更操作の排他、1要求の実行回数制御、切断時のrollback／cleanup、正常shutdown、既存domain固有のtransaction回復・重複配送対策。
- 仕様と計画: ADR 0049と `.codex/plans/2026-09-05-issue-888-cli-command-parity-v1.md` を改訂した。追加のユーザー回答でCLIは未リリース・後方互換不要と確認したため、当初記載した版更新・旧版処理・旧台帳移行は取り止め、現行protocol v1の定義・実装・schema・testsを直接修正する。GUIと既存domainの契約は維持する。
- 実装状態: 承認後に操作キー・台帳の実装とbackupへの収録・復元時の生成を撤去した。対応表の `scope_revision` も更新済み。後述する過去の成功記録と、以下の改訂後の検証を区別する。
- 次の一手: 全131操作の登録を完了した。残る権限遷移の検証、全体validation、文書・Issueの整合、固定PR headの独立監査へ進める。復元後の待機可否についての質問は撤回し、決定待ちのBlockerとして扱わない。

### 操作キー・台帳の撤去（改訂後の検証）

- `main`／request／metadata／schema／registryから操作キーとその必須条件を撤去し、protocol v1を直接更新した。版更新・旧形式の移行処理は追加していない。
- dispatcherは変更操作の種類に基づいて排他と実行中taskの所有を行い、handlerを1回起動する。同一request ID・本文の別入力でも再実行を抑止しない。実行中taskはEphemeralで、終了時は後処理を待つ。
- `host/idempotency.rs` と全production callerを撤去した。削除した9件の旧台帳テストは、ユーザーが撤回したkey再利用・保持期間・復元保護の契約を固定していた。代わりに要求ごとの起動回数、公開投稿・DMの2入力、切断後の後処理、操作履歴を生成しないbackup往復を検証する。既存domainの配送・一意性の処理は変更していない。
- 変更前: `explicit_inputs_execute_once_each_without_an_operation_key_or_host` は `validation_failed`（操作キー必須）で失敗。`validation_only_checks_all_restored_inputs_without_creating_runtime_artifacts` は復元検証時の台帳生成を検出して失敗。
- 変更後: `cargo test -p kukuri-cli --lib --test single_execution --test dispatch_lifecycle --test content_live --test direct_messages --test private_channels` は42件成功。`cargo test -p kukuri-desktop-runtime --lib device_backup` は21件成功。復元journal・同意待ち・失敗時rollbackの既存テストも含む。
- LinuxではCLI lib 30件・bin 9件・上記integration 13件の計52件が成功。`socket_inputs_execute_once_each_and_timeout_never_reexecutes` は実Unix socketで同一入力2回とtimeoutした別入力のhandler起動が各1回であることを確認した。これは複数の実daemonを起動するE2Eの代わりにはしない。
- 既存profileのファイル削除や変換は行っていない。全体validation、独立監査、CIはまだこの記録の成功範囲に含まない。

### network・Community Node・Metaverseの追加（改訂後の検証）

- network基本8操作とCommunity Node 24操作を共有runtimeへ接続した。未同意nodeでは認証・招待コード・検索・フィードバックからのHTTP送信が0件。明示的同意後の認証、401時の共有runtimeの再認証、policy更新後の本文送信・自動認証0件をテストした。招待コードは専用frameへ分離し、保存済みのリモート診断も通常JSONへ生で転記しない。
- テスト用Community Nodeの同意status route不足を修正し、2件成功。その後5回連続実行でも成功した。実Community Node接続や複数node混在を、このmockだけで検証済みとはしない。
- `network_commands_preserve_identity_and_subscription_changes_after_restart` で、CLIから有効化した購読が再起動用の保存状態に入らないことを先に検出した。共有hostの購読保存へ接続後に成功。identity維持、有効化の復元、解除済み購読の非復活、無効化状態の保持を確認した。
- Metaverse／Dome 22操作とschemaを登録した。Dome作成が未登録で失敗するテストから、作成・更新・所有者hosting開始・Join・snapshot／resync・layout no-op・終了まで成功。既存Game／Metaverse wire fixtureも維持。asset入力は既存のfile／hash検証を再利用する。
- lease／activation／closeの署名済みrecordはADR 0038のSpatialContext replica上の製品データ。CLIへ新たにaccess proofを入力させず、Community Nodeへの短命proofの生成と送信は既存runtime内に維持する。token・秘密鍵・private資格情報の入出力とは区別する。
- WindowsのCLI lib 29件・Metaverse integration 1件とclippy（全target、warningをerror扱い）が成功。対応検査は4成功・1失敗で、残る12操作を正しく検出した。全22操作の正常・拒否経路と実process E2Eは未完了。

### lifecycle・実daemonの接続（改訂後の検証）

- lifecycle／identity／backupの12操作を追加し、対応検査5件が成功。GUI登録137件のうち対象131件と除外6件、CLI固有4件を同じ登録簿から列挙する。
- `session.rs` は共有の同意・復元処理を利用し、同意前はruntimeを起動しない。復元後は同意待ちに戻し、明示的な再同意後に時間待ちなしで操作できる。鍵とpassphraseは専用frameで渡し、通常JSONには暗号化export本文も出さない。
- account切替は共有hostを利用する。backup作成の失敗・取消、復元のcommit前の失敗では、rollbackを確認して元のruntimeを再構築する。commit後は旧runtimeを再公開せず、共有journalによる回復へ接続する。backup取消は変更操作の待ち行列を待たない。
- Linuxの `tests/process_e2e.rs` は実行ファイル、Unix socket、複数の実daemon、専用の一時profileを使う。5件を個別に実行して成功した。公開投稿・返信、DM本文、private招待・投稿・rotate・leave、Live、Game、Dome hosting、再起動後のidentity・購読・DMを確認した。Dome接続申請・相手側承認・解除を追加した3台テストも成功した。
- 鍵export／preview／import／切替、切替先DB破損時の元account維持、暗号化backup往復、復号失敗時の無変更、復元直後の操作、実行中backup取消後のruntime復帰を実CLIで確認した。既存profileは変更していない。
- 実CLIとHTTP mockでnode Aの同意・401再認証・再起動後の規約失効を確認した。未同意node BへのHTTPは0件で、Aへの同意をBへ流用しない。これは実Community Nodeの配布・運用検証ではない。
- `cargo xtask check` は成功（fmt、workspace clippy、Tauri check、frontend lint／typecheck）。Linux全CLIテストは旧4件固定の一覧テストが失敗したため、2ページの内容一致と全135件を検証する形に更新し、その8件のdaemonテストは成功。全CLI・workspace全体の最終結果は実行中で、まだ成功とは判定しない。

### 全体検証で見つかった差分と切り分け

- 招待export4操作は既存owner処理でepochを更新するため、Read登録を検出する失敗テストを置き、Writeへ修正した。旧registryのSecret出力をReadへ限定する制約を撤去した。操作履歴の撤去により、変更の結果として生成したSecretを永続cacheへ収録する必要はない。専用frame・事前宣言・通常JSON非混入・エラー秘匿は維持する。
- `disconnected_secret_output_mutation_finishes_without_reexecution` を追加し、切断後もSecret出力を伴う変更操作の後処理が1回完了することを確認した。Linuxのcontent 5件・dispatch lifecycle 3件は成功。
- 同本文・同秒のpostは既存の署名canonicalにより同IDとなる。CLIにnonce・待機・独自の一意性規則を追加せず、2投稿を数えるE2Eの入力を別秒へ配置した。handler回数の検証は同一入力・同一IDの要求2件で別に固定する。
- Windowsの全体並行実行ではCN認証statusと購読復元のテストが一度ずつ失敗した。それぞれ単独の20回連続実行では成功した。新規CLI integrationも実runtimeを起動するため、nextestの既存 `iroh-integration-serial` groupへCLIを追加し、workspace全体を再実行している。skip・assertion削減・CI jobの除外は行っていない。
- Linuxの実daemon E2E 5件は一括で成功。その後private rotate後の投稿と退出、DM offline配送・ローカル削除の検証を追加しており、追加後の最終結果は別途記録する。
- 招待importのreplica同期timeoutを `internal_error` とする不足を失敗テストで再現し、`network_unavailable` へ分類した。鍵世代不一致は `authorization_failed` とする。テスト呼出元は同期未完了だけを期限付きの別入力で再試行する。CLI／daemonには要求再送を実装していない。
- Dome assetのfile取込、部屋内イベントの発行／一覧、別topicへのDome移設はCLI handlerを通すintegrationで成功。既存GUI／domainの署名・asset・move契約は変更していない。
- Issue #888／#885の現行本文を承認済み仕様へ更新した。GitHubにopen milestoneは存在しないため、新しいmilestoneの作成は行わない。リポジトリ上の現行builder previewスコープは維持する。

## 状態遷移と検証の対応

| ID | 事前状態と入力 | 期待する結果・禁止する副作用 | 検証 |
| --- | --- | --- | --- |
| TR-1 | public購読済みA／B、投稿2入力と返信 | 入力ごとに処理し、各objectは1件。private本文をpublicへ出さない | `content_live`、3台の実daemon E2E |
| TR-2 | 相互フォロー済みA／B、未承認C、online／offline／再起動 | 本文は参加者のローカルJSON。Cの送信は拒否。削除はローカルだけ | `direct_messages`、実daemon E2E（offline配送・受信側だけの削除を含む） |
| TR-3 | owner／参加者／未招待、epoch更新と退出 | 既存handoffに従い参加者へ更新、退出後はjoinedから除外。資格情報を通常JSONへ出さない | `private_channels`、runtime private tests、実daemon E2E |
| TR-4 | owner／非owner、Live／Game／Dome | 所有者の操作・接続合意を共有runtimeへ渡し、非owner更新を拒否 | `content_live`、`metaverse`、実daemon接続申請／承認／解除、既存Dome scenarios |
| TR-5 | 未同意、稼働中、backup、誤passphrase、破損account DB | 未同意時runtimeなし。失敗時は元accountを維持。復元後は再同意後すぐ操作可能 | `session`、実daemon key／switch／backup E2E、runtime device_backup |
| TR-6 | node Aのみ同意、B未同意、401、規約更新後の再起動 | BへのHTTP 0件。共有runtimeの401再認証を維持。規約失効後に本文送信・自動再同意しない | `community_node`、実daemon＋HTTP mock |
| TR-7 | guard／Secret入力／owner／scopeが不成立 | 対象handlerまたはdomainが拒否し、秘密情報を返さない | dispatcher guard／secret tests、Game非owner、DM未承認、private未招待、同意前投稿 |
| TR-8 | 同一IDの独立した入力、実行中の切断／timeout／取消／終了 | handler最大1回、別入力の統合なし。後処理を破棄せず操作履歴を残さない | `single_execution`、`dispatch_lifecycle`、実socket counter、実backup取消、accept失敗時の完了待ち |

## 入口の対応表

正本の登録点は `apps/desktop/src-tauri/src/lib.rs` の `tauri::generate_handler!`。
`crates/kukuri-cli/command-parity.json` が全member名とCLI名または除外理由を保持する。
`crates/kukuri-cli/tests/command_parity.rs` はRust構文を解析して登録点を再列挙する。

| 分類 | 件数 | 状態 |
| --- | ---: | --- |
| GUI登録総数 | 137 | 基準と一致 |
| CLI対応対象 | 131 | public系35件、DM7件、private channel15件、Live5件、Game3件、network基本8件、Community Node24件、Metaverse／Dome22件、lifecycle／identity／backup12件を登録。登録数は全受入条件の完了数ではない |
| OS通知固有 | 4 | 表示・権限読取・権限要求・背景通知設定を除外 |
| GUI表示状態の復元受渡し | 2 | CLIに適用先がない表示状態の読取・受領確認を除外。バックアップ本体の保存・復元は対象 |
| 未分類 | 0 | 適合判定・入口からsinkの確認は別途必要 |

| ID | 対象入口群 | 共有境界とsink | 保護する条件 | 遷移 |
| --- | --- | --- | --- | --- |
| INV-1 | CLI要求の実行・終了、startup、app consent、identity、backup | dispatcher／host／account registry／鍵保存／復元journal。旧ローカル台帳は撤去対象 | 同意・年齢申告、排他、1入力1回、秘密情報、取消とrollback | TR-5、TR-7、TR-8 |
| INV-2 | posts、reactions、profile | DesktopRuntime／AppService／docs・blob・DB・gossip | scope、author権限、入力と制御の分離 | TR-1、TR-7、TR-8 |
| INV-3 | direct_messages | DM runtime／暗号化docs・blob・配送状態 | 相手と会話の境界、本文のpublic漏えい禁止 | TR-2、TR-5、TR-7、TR-8 |
| INV-4 | private channel・invite・grant・share | private replica／capability store／namespace secret／epoch | audience、credential、失効、失敗時cleanup | TR-3、TR-5、TR-7、TR-8 |
| INV-5 | Live・Game | signed state／docs・presence・gossip | owner・participant・scope | TR-4、TR-7、TR-8 |
| INV-6 | Metaverse・Dome | signed state／asset blob／hosting・lease・access proof | owner・participant・delegation・scope | TR-4、TR-7、TR-8 |
| INV-7 | network・Community Node | discovery／relay・seed／HTTP・token・per-node consent | 同意・認証・P2P経路、nodeごとの状態 | TR-6、TR-7、TR-8 |

## 最終差分の検証（2026-09-05）

- Windowsの `cargo xtask test` はRust 866件（既存の除外3件）、harness 22件、doctest、frontend 141ファイル・1,093件が完走して成功した。後から追加したprivate同期エラー分類とLinux専用accept失敗テストはLinux全CLIテストで補完する。
- 最終production差分の `cargo xtask check` は成功。workspace clippy、Tauri check、frontend lint／typecheckを含む。Linuxの `cargo clippy -p kukuri-cli --all-targets -- -D warnings` も成功した。
- `cargo xtask e2e-smoke` は成功（永続往復6段階）。`cargo xtask scenario community_node_public_connectivity` は実cn-user-api／relay・PostgreSQL・Valkeyを使用して15段階成功。試験用container・volumeは同commandの終了処理で削除した。
- accept失敗時に終了待ちを飛ばす経路を `accept_failure_waits_for_the_owned_mutation_before_shutdown` で再現して修正した。修正前は実行中handlerの完了を待たずに失敗、修正後は完了後に元のacceptエラーを返し、handler回数は1回。再現用socketは読み取り可能な非listen socketを使用する。最初のfixtureの停止済みlisten socket／未接続datagramでは期待するacceptエラーを通知できず、試験processだけを停止してfixtureを改めた。コンパイルやworkspace全体テストを途中で中断したものではない。
- Linux全CLIテストの初回最終実行では、lib 31件・bin 10件・対応表5件・各handler integrationが成功し、実processは4成功・追加したoffline DM待機1失敗。既存harnessのpair refreshと同様に、送信側statusの確認と5秒ごとの接続情報更新を試験呼出元へ追加した。DM送信は各1入力のままで、受信するmessage IDのassertionは維持する。
- 最終 `cargo test -p kukuri-cli -- --test-threads=1` は全78件成功（lib 31件、bin 10件、integration 37件）。実process E2E 5件にはoffline DMと受信側だけの削除を含む。最終差分のLinux clippyも成功。stage後の `oversized-files` と `git diff --cached --check` は成功し、既存大型ファイル14件の基準は変更していない。
- DomeのCLI正常経路はowner hosting／Join／snapshot／layout／asset／event／moveと、別ownerへの接続申請・承認・解除で検証する。transitionの署名・reservation・commit・同時在室禁止は既存 `desktop_smoke_metaverse_dome_transition` が成功した。既存DesktopRuntimeは他端末のowner-device宛transitionを許可せず、Community Node宛には内部でaccess proofを生成する。この共有挙動は変更せず、CLI固有の遠隔hosting機能は追加しない。

### 共有処理・副作用の確認方法

登録点の構文解析と対応表を起点に、CLIの `commands::{content_social,direct_messages,private_channels,live_game,live_metaverse,network_community_node,community_node,lifecycle}` の全match armから同名の共有DTO／runtime呼出しへ辿る。逆方向は共有runtimeの該当method、AppServiceのdocs／blob／DM／private／Domeの書込み、Community Node HTTP wrapper、hostのaccount切替、backupのprepare／install／commitからcallerを照合する。Tauri adapterとbackground呼出しは既存のまま、CLIからの追加callerはこの8moduleとsessionに限定される。

- INV-1: `session.rs`／`session/backup.rs` → 共有consent・restore lifecycle・host。`daemon::wait_for_shutdown` → connection drain → `Dispatcher::finish_operations` → session shutdown。backup取消だけは変更操作の排他を待たない。旧台帳識別子の残存参照は非生成を検証するtestのみで、製品domainのDome Connection固有idempotencyは変更しない。
- INV-2〜6: `commands::runtime` のHostReadyと共有runtime／AppServiceがscope・author・audience・epoch・ownerを判定する。Secretの4種exportはowner helperによるepoch変更を伴うためWrite。file/hash入力以外のdomain DTO・signed recordを複製しない。
- INV-7: CLI → 既存のper-node consent／認証／HTTP処理。node Aの検証済み状態をnode Bへ適用する共通化は追加しない。購読変更はhostの保存処理を通し、既存transport／relay／seed選択と401再認証は変更しない。

これは実装者の照合方法と検証結果であり、独立監査PASSの代わりにはしない。

## 再開時の見直し

モデル変更後、未コミット差分を基準commitと照合した。既存の草案は35件のruntime呼出しだけで、schemaは説明文のみ、残り3モジュールは空だった。これを完了実装とは扱わない。

### 再現して修正した退行

草案では `HostGuardEvaluator` がhostの存在だけでAccount／Consent／Audience／Credential／DomainAuthorizationをすべて許可していた。未設定のguardは拒否する既存挙動へ戻した。

- 修正前: `unconfigured_domain_guard_rejects_even_when_host_is_ready` が `Account` の許可を検出して失敗。
- 修正後: 同テスト成功。大型ファイルの成長を避けるため、以後の回帰テストは `tests/dispatch_lifecycle.rs` で公開dispatcher境界を検証する。

### 固定範囲内の実装課題

以下は旧revisionの台帳契約を前提とした読み取り専用の境界確認であり、最終PR監査ではない。項目1の管理台帳追加と項目2の台帳完了要件は、上記ユーザー決定により撤回する。切断時の後処理とprivate情報の分離は引き続き必要となる。

1. T2: 同意前の変更操作はready host／account DBを要求する既存dispatcherへ単純登録できない。profile管理操作の台帳bindingとbackupへの保存を既存台帳契約の範囲で接続する。account切替前後の再送も同じ操作を識別する。
2. T2: backup cancelは通常mutationの直列lockを待たせない。socket timeoutで呼出しfutureが破棄されても、install／rollback／同意gate／台帳完了まで処理を所有する。shutdown時も安全な終端を待つ。
3. T4: private importの既存Previewは `namespace_secret_hex` を含む。通常JSONと台帳へそのまま保存せず、安全な結果へ明示変換する。import途中の切断でもprivate replica secretのcleanupを完了する。

当時はこれらをAC-2／AC-5／AC-6、INVAR-1〜3、INV-1／INV-4のRequiredとしてT2／T4へ統合した。現行の範囲は改訂後の計画を正とし、管理台帳やその復元journalを追加しない。

### DM本文の返却方法

2026-09-05のユーザー回答で、DM本文は権限のあるローカルCLIへJSONで返すと確認した。
Privateな製品データとSecretな鍵・招待資格情報を区別する。本文をpublic gossipや診断出力へ漏らさず、Secretは専用frameへ分離する。承認済みプランT4の曖昧な「通常JSONへの出力禁止」はこの区別へ修正した。

### 共有処理の先行抽出

PR [#899](https://github.com/KingYoSun/kukuri/pull/899)で同意受付・復元journal・操作guardの純粋な処理をdesktop-runtimeへ抽出した。Tauriの通知・状態公開などのadapterは残している。
独立監査対象 `79f2737a34ffe160411a3a7d782b2be625e07e33` はPASS、全必須CI成功後にマージした。
マージcommit `e4b37b5958a2fb7f66cd77cd76f61cf12280ae3a` と監査対象のtreeはともに `dc79a51be0f548c1ec4638fbdf555288e0585fb0` で一致する。
詳細は [抽出の記録](2026-09-05-issue-888-shared-lifecycle-extraction.md)。このPASSは#888全体の完了判定ではない。

### DM・private channelの実装経過

- DMは本文、返信先、配信状態を共有viewのJSONで返す。添付入力は絶対path・BLAKE3 hash・byte_size・mimeの明示参照を共有DTOへ変換し、ファイルの変更を検出する。
- private channelのinvite／grant／shareはUTF-8の専用secret frameへ分離した。importが返すnamespace secretを通常JSONへserializeせず、ChannelAccessTokenPreviewに対応するフィールドだけを返す。
- 既存runtimeが所有する相互フォロー・audience・owner・epochの判定を呼び出す。未実装の共通guardを一括で許可する変更は行わない。
- 共有DTOのnullableな出力は、protocol v1の型union非対応を維持し、型のannotationとobject／arrayの構造制約を明示する。入力の省略可能フィールドは省略を用い、型付きDTOでも検証する。
- 草案のserde／runtimeエラーが入力本文を診断へ転記する問題を再現し、入力値を含まないエラーへ修正した。Community Nodeの型付きエラーはHTTP status・再試行待機時間を残し、リモートのmessage／未知のcodeを転記しない。

### public・Live・Game・メディアの実装経過（旧契約時点の記録）

- public系35件の入出力schemaを、共有request／viewのフィールドに対応させた。分岐型の条件は共有DTOでも検証する。
- public系の草案が未設定の共通DomainAuthorizationで常に拒否されることを再現し、既存runtimeが所有するauthor／audience判定へ到達する登録に修正した。共通guard自体の未設定時拒否は維持し、両方を回帰テストする。
- Liveの開始・参加・離脱・終了・一覧、Gameの作成・更新・一覧を接続した。Metaverse／Domeのcommandは未着手。
- Gameの出力schemaは既存のscore／Metaverse双方のwire fixtureを通し、共有viewの入れ子フィールドも保持する。作成・更新の同じkeyによる再送、不正な参加者一覧での状態不変を検証した。
- 2つの一時clientでGameの伝播と所有者以外からの更新拒否を検証した。既存所有者チェックのエラーが汎用エラーになることを再現し、固定エラーだけを `authorization_failed` へ変換した。拒否後の同じkeyは既存台帳どおり `operation_outcome_unknown` となり、両clientのmanifest／投影は不変。GUIと共有domainの実装は変更していない。
- 投稿・プロフィール画像・custom reactionの添付入力もファイル参照へ変換する。メディア取得は明示した絶対pathへ新規出力し、既存ファイルを上書きせず、JSONにはpath／hash／mime／byte_sizeだけを返す。

### T2の復元台帳案の撤回

当初はprofile管理操作用のroot台帳を追加し、account台帳とともに復元する案を検討した。その案では復元後の再同意にも既存の時刻保護が掛かるため待機可否を質問したが、ユーザーとの要件確認により入力間の重複排除そのものがCLIの責務外と決定した。
したがってroot台帳の追加・復元は実装せず、既存account台帳への依存も撤去する。復元後の同意・年齢申告は既存製品の要件として維持するが、CLI独自の待機時間は設けない。

## 旧契約での検証記録

以下は責務見直し前のコードに対する実行結果。新契約の検証記録は実装後に分けて追加する。

| 検証 | 結果 |
| --- | --- |
| 未設定guardの回帰テスト | 修正前失敗、修正後成功 |
| `cargo test -p kukuri-cli --test command_parity` | 4成功・1失敗。分類・追加削除重複・除外理由・構文解析は成功。実登録簿との照合は未実装66件を正しく検出 |
| Tauri `restore_lifecycle` 変更前テスト | 6件成功。抽出後の共有moduleでも6件成功 |
| Tauri `--lib`／共有hostテスト | 29件／28件成功（先行抽出） |
| 接続切断時のmutation完了テスト | 修正前失敗、修正後成功。接続futureを破棄しても後処理・台帳完了を継続 |
| `cargo test -p kukuri-cli --test direct_messages` | 3件成功。DM登録、拒否時会話row不変、本文のローカルJSON返却と同じkeyでのmessage重複0件 |
| `cargo test -p kukuri-cli --test private_channels` | 2件成功。secret metadata、invite／namespace secretのJSON・台帳非残留とimport再送 |
| `cargo test -p kukuri-cli --lib` | 29件成功。診断漏えい2件は修正前失敗を確認。ファイルhash固定、Community Nodeエラー変換、Gameの既存wire fixtureも成功 |
| `cargo test -p kukuri-cli --test content_live` | public投稿／返信、Live、Game作成・更新と不正roster拒否、別clientの所有者拒否、メディア往復の5件成功。public登録の拒否、Game未登録、所有者拒否のエラー分類は修正前失敗を確認 |
| `cargo test -p kukuri-cli --test dispatch_lifecycle` | 共通guard拒否、切断後完了の2件成功。public登録修正後も維持 |
| マージ済み共有hostテスト | `e4b37b59…`の共有hostコードで28件成功 |
| `cargo xtask oversized-files`、`git diff --check` | 成功。既存大型ファイル14件の基準は変更していない |
| `cargo clippy -p kukuri-cli --all-targets -- -D warnings` | 成功。新規schema定義の `unwrap_used` 4件を修正後に再実行 |
| 全体validation、Linux実socket E2E、最終独立監査、CI | 未実行 |

CI成功だけではCloseしない。固定AC／INVARの証跡とPR headの独立監査PASSを確認してからマージし、merge tree照合後にIssueを更新する。

## 仕様・計画改訂時点の記録（過去）

承認前の改訂時点では文書と作業プランだけを変更し、CLIのparser／送信処理、dispatcher／実行task、台帳とbackupの参照先を確認して撤去範囲を記録した。承認後の実装と検証は本書上部の改訂後記録を参照する。

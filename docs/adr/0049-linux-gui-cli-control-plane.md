# ADR 0049: Linux GUI配布とCLIのローカル制御経路

## Status

Accepted

2026-09-05改訂: CLIの責務を1入力からの重複実行防止に限定するユーザー決定により、
入力間の重複排除、永続的な冪等性台帳、復元後の時刻による実行制限を撤回した。
改訂前の実装・検証記録は `docs/progress/2026-09-05-issue-887-cli-protocol-v1.md` に保持する。
この改訂の実装・検証は #888 で行う。仕様の改訂だけを実装完了とは扱わない。

## Context

Issue #885は、Linux x86_64 GUIをAppImageとして配布し、GUIとは別profileを所有する
単一所有の常駐プロセスと薄いCLIを介してKukuriクライアントを操作できるようにする。

この追加は既存の投稿、DM、private channel、Live、Game、Metaverse／Dome、Community Nodeの
canonical sourceを変更しない。一方で、CLI profileの購読期待状態、ローカルIPC、command登録簿、
要求単位の実行状態、複数platformの配布成果物という新しいデータ境界を持つため、ADR 0002に
従って実装前に分類する。

## Feature Data Classification

### Linux GUI／CLI配布成果物

- Feature 名: Linux GUI／CLI Preview Release成果物
- Durable / Transient: Durable。公開したPreview Release単位で保持する
- Canonical Source: 配布対象tag／source commitと、そのcommitから検証・署名・集約した同一実行のGitHub Preview Release成果物一式。`latest-preview.json`は公開した成果物のversion、target、URL、signatureを参照する
- Replicated?: Kukuri protocol上はNo。GitHub Release／CDN上の配布copyはapplication replicaとして扱わない
- Rebuildable From: 固定source commit、固定したtoolchain／runner、成果物一覧、署名鍵から論理的に再生成可能。ただしtimestampや署名を含むbyte単位の再現性は完了条件にしない
- Public Replica / Private Replica / Local Only: 公開配布物。Kukuriのpublic／private replicaは増やさない
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要
- 必須 contract: x86_64 AppImage、x86_64／aarch64 CLI archive、checksum、必要な署名、Windows成果物、platform manifestを同一sourceから完全に生成し、一部欠落時はmanifest／Releaseを公開しない。署名用secretを成果物、cache、log、untrusted eventへ渡さない
- 必須 scenario: Ubuntu 22.04／Debian 12における配布済みAppImageの実環境smoke test、x86_64／aarch64 CLI smoke test、Windows回帰、不正なupdater署名、成果物欠落、取得失敗を含む検証環境での更新

### CLI専用profileと購読期待状態

- Feature 名: CLI専用クライアントprofile
- Durable / Transient: Durable。所有lock、process／session、socketはTransient
- Canonical Source: GUIと分離したprofile directory内のaccount registry、identity backend、既存featureごとのcanonical store、およびprofile内の購読期待状態
- Replicated?: 既存製品データだけが各featureの既存契約に従ってreplicateされる。profile管理情報と所有状態はNo
- Rebuildable From: 既存製品データは各featureのcanonical sourceから復元可能。identityと購読期待状態はそれぞれのローカル永続状態がなければ再構築しない
- Public Replica / Private Replica / Local Only: 制御情報はLocal Only。製品データの区分は既存featureの分類を維持する
- Gossip Hint 必要有無: 制御情報には不要。購読後の製品データは既存契約を維持する
- Blob 必要有無: 制御情報には不要。製品データは既存契約を維持する
- SQLite projection 必要有無: 既存profile DBを維持する。購読期待状態の具体的な保存形式は子Issueで固定するが、別profileやglobal設定へ暗黙適用しない
- 必須 contract: GUI profileとのpath／identity非共有、一profile一所有者、同profileの二重起動拒否、別profile同時起動、restart後のidentity／購読期待状態復元、同意前network I/O 0件
- 必須 scenario: 新規profile、同意待ち、通常起動、restart、signal、同一profileの競合、別profileの同時実行、Secret Service不在

### ローカルsocketの要求／event stream

- Feature 名: 常駐プロセスのローカル制御protocol
- Durable / Transient: Transient。request／response／NDJSON event stream自体は保存しない
- Canonical Source: 常駐プロセスの版管理されたcommand登録簿、schema、dispatcher metadata。CLI parserや表示文言を正本にしない
- Replicated?: No
- Rebuildable From: command登録簿とruntime eventから再生成する
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要。大容量mediaはprotocolへbase64で埋め込まず、明示したローカルfile／blob参照を返す
- SQLite projection 必要有無: 不要
- 必須 contract: Unix socketのみ、runtime directory 0700、socket 0600、接続元UID一致、TCP listen 0件、版管理されたJSON／NDJSON、stdout／stderr分離、型付きerror、容量／timeout／backpressure上限、remote contentと制御情報の分離。v1は通常frameとsecret frameを各1 MiB以下、既定timeoutを30秒、指定可能な上限を5分、同時接続を64件以下とする。timeoutは接続、frame入出力、dispatcher、event stream全体へ適用する。stream継続はresponseの`more`で示す。secret frameはrequest headerのschema／profile／version／guardと出力先宣言を検証した後、相関付きの受信許可を返してからだけ送受信する
- 必須 scenario: 正常／不正なenvelope、protocol不一致、常駐プロセス不在、timeout／SIGINT、低速な購読側、上限超過入力、接続元UID不一致、secret／untrusted contentの漏えい検査

### command登録簿と操作安全情報

- Feature 名: CLI command登録簿と操作安全情報
- Durable / Transient: 実行ファイルに含まれる再生成可能な定義。要求単位の検証結果はTransient
- Canonical Source: 常駐プロセスのcommand登録簿とdispatcher metadata。CLI parserや呼び出し元の状態を正本にしない
- Replicated?: No
- Rebuildable From: sourceとschema生成処理から再生成する
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要
- 必須 contract: read／write／destructive／secret-bearing metadataをdispatcherと同じ定義から生成する。操作可否は既存のaccount、同意、audience、credential、domain authorizationで判定する。command schemaの検証subsetは`type`、`properties`、`required`、`additionalProperties`、`items`、`enum`、`const`、数値／文字列／配列のmin/max制約とannotationに限定し、未対応keywordは登録時に拒否する。request payloadの契約に合わせ、input schemaのroot `type`は未指定または`object`に限定する
- 必須 scenario: command登録簿とdispatcherの不一致、既存domain authorizationによる許可／拒否、argv／payload／remote contentを制御情報として誤解釈しないこと

### 要求単位の実行状態

- Feature 名: CLI要求の実行と終了処理
- Durable / Transient: Transient。要求と実行中taskの対応はメモリ内だけで保持する
- Canonical Source: 常駐プロセスが所有する要求単位のtask。製品データの正本は既存domainのstoreとする
- Replicated?: No
- Rebuildable From: No。終了した要求をrestartやbackupから再生成・自動再実行しない
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要。入力間の重複排除のための台帳・payload digest・結果cacheを作成しない
- 必須 contract: 1回の入力から受け付けた1要求について、CLI／dispatcherはcommand handlerを最大1回だけ起動する。timeout、応答喪失、切断、再接続、restartを理由に同じ要求を自動再実行しない。別々に入力された要求は内容やrequest IDの一致でまとめず、それぞれ既存domainの規則に従って処理する。request IDは応答との対応付けにだけ使用する。変更操作の排他と容量制限を維持し、切断によって進行中のrollback／cleanupを破棄しない。成否不明は成否不明として返し、未実行や取消成功と断定しない
- 必須 scenario: 1入力1回のhandler起動、同内容の2入力を別々に処理、拒否時handler起動0回、timeout／切断時の再起動0回と後処理完了、正常shutdown、restart後の自動再実行0回、復元直後の明示的な新規操作

### CLI protocolの改訂境界

- CLIは未リリースのため後方互換は不要とする。#888で現行protocol v1の定義を直接修正し、この変更のための版更新・旧版処理・移行層は追加しない。
- Secret出力の有無と変更種別は独立に定義する。招待exportのように既存domainで鍵世代を更新する操作は変更操作とし、秘密値を永続的な結果cacheへ保存せず、専用frameでのみ返す。切断後も変更操作の後処理を完了する。
- `--idempotency-key`、requestの `idempotency_key`、metadataの `idempotency_required` を定義・実装・schema・testsから撤去する。
- 通常frame／secret frame、Unix socket、入力サイズ・timeout・接続数の上限、既存domainの署名／wire／認可規則はこの変更で拡張しない。

## Decision

- GUIとCLIはprofile／identityを共有しない。GUI processと常駐processが同じprofileを同時所有する方式は採用しない。
- 常駐プロセスだけがprofile内のDesktopRuntimeを所有し、薄いCLIはローカルUnix socket越しにrequestを送る。公開TCP制御APIは提供しない。
- app／Community Node同意、age gate、restore gateを共通hostで共有し、成立前はruntime、scheduler、remote取得、background通知を開始しない。
- protocolの正本は常駐プロセスのcommand登録簿／dispatcherとし、schema／command metadataを同じ定義から生成する。
- account、同意、private audience、credentialなど既存の製品側guardは、GUIと同じ意味で常駐プロセス側にも適用する。
- Tauri updater署名は必須とする。AppImage埋込みGPG署名は別の配布契約として扱い、採否を#889の計画承認前に固定する。
- Ubuntu 22.04をLinux GUI build基盤とし、Ubuntu 22.04／Debian 12のX11／XWaylandを実環境smoke test対象にする。

## バックアップ／復元境界

- CLI profileのidentity、既存製品データ、購読期待状態はprofile backup対象に含める。CLIの操作台帳は作成・更新・移行しない。
- 所有lock、socket、PID、実行中session、bearer token、endpoint secretは移行しない。
- app／Community Node同意とage attestationは既存のbackup／restore契約どおり移行せず、restore後に再設定を要求する。
- 復元は、移行対象のローカル状態をバックアップ生成時点へ戻す。既に他端末やCommunity Nodeへ伝わったデータは巻き戻さない。復元をまたぐ入力間の重複排除は行わず、復元後の経過時間によるCLI操作の保護期間を設けない。必要な明示的再同意等が成立した操作は、待機時間を追加せず実行できる。
- 未リリースCLIの旧台帳・旧backupを救済する互換処理は追加しない。GUIの既存バックアップ契約はこの変更の対象外とする。

## セキュリティ境界

- runtime directory 0700、socket 0600、接続元UID検証は、別OS userからの接続と偶発的なprofile間接続を防ぐ。
- 同じOS userで動くprocessは同じローカル権限を持つ。分離が必要な運用では別OS userと別profileを使用する。

## Consequences

- CLI追加のために既存featureのdocs／blobs／gossip／SQLite、署名canonical、private audience、P2P三経路を変更してはならない。
- 常駐プロセス、protocol、command登録簿、要求単位の実行状態はLocal Onlyの制御経路であり、Community Nodeやpeerへ制御指示を送信しない。
- untrustedなpost／DM本文をcommand、profile、file path、log指示として解釈しない。
- E2Eで使う一時profileはtest harnessが作成・破棄する。
- 共通host抽出では、#855／#857後のrestore／consent／background actionを含む全callerを現行基準から再生成する。

## 参照

- [Tauri AppImage](https://v2.tauri.app/distribute/appimage/)
- [Tauri Updater](https://v2.tauri.app/plugin/updater/)
- [Tauri Linux signing](https://v2.tauri.app/distribute/sign/linux/)
- Issue #885、#886、#887、#888、#889、#890

# 2026-06-11 リリース準備 実行計画

## 概要

- この計画は、ビルダープレビューを「配布できる」「更新できる」「問題報告を回収できる」状態にするためのリリース準備計画です。
- 既存のローカル通知 inbox は、プロダクト内アクティビティの永続ローカル inbox として維持します。
- OS 通知は既存の通知 inbox とは別の機能単位として実装します。既存の `NotificationRow` / `NotificationView` はアクティビティ履歴の正本、OS 通知はユーザーの許可に基づく注意喚起面として扱います。
- アップデーターとアップデート通知は、リリース基盤の一部として最優先で実装します。OS 通知とは別に、アプリ内の更新状態、更新バナー、設定画面上の更新表示を持ちます。
- 初回プレビューの配布対象は Windows インストーラーとし、Linux はソース起動の代替導線のままにします。

## 現状

- Windows リリースワークフローは存在しており、タグまたは手動実行により、検証後に Windows パッケージを作成して GitHub Releases の配布物として公開します。
- 既存の `.github/workflows/kukuri-release.yml` は、Linux 検証後に Windows NSIS を生成して GitHub Release へ添付するところまでは到達しています。一方で、updater 用 `.sig`、`latest-preview.json`、チェックサム、artifact provenance、draft smoke 後の公開昇格はまだ明示的な成果物になっていません。
- `workflow_dispatch` で tag input を受け取れますが、リリース対象 ref の checkout、タグ形式、タグとアプリ内バージョンの一致確認は release gate として固定する必要があります。
- Windows のバンドル対象は NSIS インストーラーです。
- `tauri-plugin-updater`、更新マニフェスト、更新署名ファイル、更新 UI はまだリリース面に含まれていません。
- ローカル通知 inbox v1 は、メンション、返信、リポスト、引用リポスト、ダイレクトメッセージ、フォロー通知のアクティビティ inbox として存在します。
- OS トースト通知、プッシュ通知、通知の破棄、通知のアーカイブは、ローカル通知 inbox v1 の範囲外です。
- Community Node の診断表示とトラブルシューティング文書はありますが、ユーザーが GitHub へのフィードバックに貼れる秘匿情報除去済み診断レポートは、まだ明示的なリリース条件になっていません。
- 本番用 CSP、Windows コード署名、アップデーター署名、インストーラーの信頼性説明は、リリース条件として固定する必要があります。

## リリース準備の原則

1. **更新可能性を先に固める**
   - プレビュー配布後の修正配布を成立させるため、インストーラー公開より先にアップデート経路を完成させます。
   - 手動ダウンロードだけに依存しません。

2. **通知の役割を分離する**
   - プロダクト内アクティビティは、既存のローカル通知 inbox に残します。
   - OS 通知は、ユーザー許可、フォアグラウンド/バックグラウンド方針、静音設定、権限拒否時の代替表示を持つ別レイヤーにします。
   - アップデート通知は、リリース/更新状態として扱い、アクティビティ inbox の自動既読挙動から独立させます。

3. **問題報告できる状態を完了条件に含める**
   - 通信が安定しても、プレビューではユーザー環境に由来する失敗が残ります。
   - 秘匿情報除去済み診断レポートの出力を、初回プレビューの必須要件にします。

4. **identity とローカルデータを壊さない**
   - アップデート、再インストール、マイグレーション、keyring fallback はプレビューの信頼性に直結します。
   - リリース検証は新規インストールだけでなく、旧版からのアップデートも必ず含めます。

5. **セキュリティ姿勢をリリース設定として固定する**
   - 開発時の利便性をそのままリリースに持ち込みません。
   - CSP、deep link の検証、署名済み更新ファイル、署名済み Windows インストーラーをリリース条件に含めます。

6. **GitHub Actions をリリース成果物の正本にする**
   - 初回プレビューでは、手元ビルドや差し替え済みファイルではなく、GitHub-hosted runner が生成した draft release asset を検証対象にします。
   - 既存の `cargo xtask desktop-package` と `kukuri-release.yml` を拡張し、初回から `tauri-action` への全面移行は行いません。
   - リリースは `validate -> package -> manifest/checksum/provenance -> draft release -> manual smoke -> publish` の段階に分けます。

## マイルストーン完了条件

ビルドは次をすべて満たした時点で、リリース準備完了とします。

- [ ] Windows インストーラーを GitHub Releases から取得し、新規 Windows ユーザープロファイルへインストールできる。
- [ ] インストール済みアプリが更新確認、更新あり表示、更新インストール、再起動、ローカルデータ保持まで完了できる。
- [ ] 更新ファイルは署名され、インストール前にアプリ側で検証される。
- [ ] Windows インストーラー/実行ファイルがコード署名されている。間に合わない場合は、未署名プレビューであるリスクと回避策をリリース文に明記する。
- [ ] リリースワークフローが、インストーラー、更新用ファイル、署名、`latest-preview.json`、チェックサム、リリースノートを一貫して draft release に公開する。
- [ ] draft release の asset を差し替えずに、Windows 10 / Windows 11 smoke 後にそのまま公開へ昇格できる。
- [ ] 可能な場合、Windows 配布物には GitHub Actions artifact attestation または同等の provenance が付与されている。
- [ ] ユーザーがフィードバック用の秘匿情報除去済み診断レポートをコピーまたは書き出しできる。
- [ ] Community Node の失敗状態が設定画面で読め、復旧操作を試せる。
- [ ] 既存のローカル通知 inbox がアップデート後も動作する。
- [ ] OS 通知を初回プレビューに含める場合、ユーザー許可制であり、ローカル通知 inbox の保存状態から独立している。
- [ ] プライバシー、データ保存、フィードバック時に含まれる情報の説明が README またはアプリ内の About/Settings から読める。
- [ ] CSP とリリース用セキュリティ設定が本番相当の安全側設定になっている。

## 作業領域

| 優先度 | 作業領域 | 状態 | 成果物 | 補足 |
| --- | --- | --- | --- | --- |
| P0 | バージョン/チャンネル運用 | 計画中 | 単一のリリースバージョン基準、プレビューチャンネル規約 | `vX.Y.Z-preview.N` を使い、`tauri.conf.json`、Tauri crate、desktop package のバージョンを同期する。 |
| P0 | アップデーター基盤 | 計画中 | Tauri updater plugin、アップデーター設定、署名鍵運用 | Rust/JS 依存を追加し、確認、ダウンロード、インストール、再起動の実行面を作る。 |
| P0 | アップデート通知 UI | 計画中 | アプリ内更新バナー、設定/About の更新状態表示 | 更新通知は既存のアクティビティ通知 inbox へ既定では保存しない。 |
| P0 | リリースワークフローの gate | 計画中 | tag checkout、version consistency、channel validation | `workflow_dispatch` の tag input と push tag の両方で、対象 ref と version を fail-fast で検証する。 |
| P0 | リリースワークフローの更新用成果物 | 計画中 | 更新用バンドル、`.sig`、`latest-preview.json`、チェックサム | `.github/workflows/kukuri-release.yml` を拡張し、draft release の asset を正本にする。 |
| P0 | Windows コード署名 | 計画中 | 署名済み EXE/MSI、CI secret 運用 | 証明書が初回プレビューに間に合わない場合は、リスク説明と手動検証条件を追加する。 |
| P0 | draft release smoke | 計画中 | draft asset からの Windows install/update smoke 記録 | release asset を差し替えず、smoke 後に公開へ昇格できることを条件にする。 |
| P0 | インストール/アップデート E2E | 計画中 | 旧版から新版への更新シナリオ | identity、DB、Community Node 設定、通知 inbox、private channel 状態を検証する。 |
| P0 | 診断レポート出力 | 計画中 | 秘匿情報除去済みレポートのコピー/書き出し操作 | アプリ版、OS、同期状態、Community Node 状態、直近エラー、設定形状を含め、秘密情報は含めない。 |
| P0 | 本番用セキュリティ設定 | 計画中 | CSP、リリース用 capability review、deep link 検証監査 | リリース設定では `csp: null` に依存しない。 |
| P1 | artifact provenance | 計画中 | GitHub Actions artifact attestation または同等の出所確認 | `id-token: write` / `attestations: write` を使える場合は Windows 配布物に attestation を付ける。 |
| P1 | OS 通知機能 | 計画中 | ユーザー許可制の OS 通知配信 | `notifications` テーブルとは別扱いにし、イベントや状態を参照するが、権限/設定状態は独立して持つ。 |
| P1 | データ安全性/リセット/バックアップ導線 | 計画中 | バックアップ、書き出し、リセットの文書または設定操作 | identity の喪失、keyring fallback、ローカル DB 場所、再インストール挙動を説明する。 |
| P1 | プライバシーとデータ保存説明 | 計画中 | README、runbook、アプリ設定上の説明 | 何がローカル保存か、Community Node へ何を送るか、診断に何を含めるかを書く。 |
| P1 | 初回起動オンボーディング | 計画中 | アプリ内チェックリストまたは案内表示 | ready、starter topic、post/reply、private channel、feedback まで案内する。 |
| P1 | DB マイグレーション安全策 | 計画中 | マイグレーション smoke、更新前バックアップ方針 | 旧版 DB fixture と、失敗時のユーザー向け表示を追加する。 |
| P1 | サードパーティ通知 | 計画中 | OSS ライセンス通知 | About 画面またはリリースノートから参照できるようにする。 |
| P1 | フィードバック窓口 | 計画中 | GitHub issue/discussion template と導線 | 診断添付、期待するフィードバック分類を事前入力する。 |
| P2 | 段階的配布/ロールバック | 後回し | 動的更新サーバーまたはチャンネル分離 | 初回プレビューでは静的 `latest-preview.json` で十分とする。 |
| P2 | クラッシュ報告/計測 | 後回し | 任意参加のクラッシュ報告 | プライバシー説明と同意設計が整うまで、ネットワーク計測は追加しない。 |
| P2 | アクセシビリティ確認 | 計画中 | キーボード/スクリーンリーダー確認表 | nav rail、dialog、通知一覧、settings drawer、更新プロンプトを含める。 |

## 実行計画

### フェーズ 0: リリース基準の棚卸し

目的: 現在の基準を固定し、リリース作業を追跡可能にします。

作業:

- [ ] 初回プレビュータグ形式を決める。例: `v0.1.0-preview.1`。
- [ ] リリースチェックリスト issue または追跡ボードを作る。
- [ ] 対象 OS を確認する。パッケージ配布は Windows 10 / Windows 11、Linux はソース起動のみとする。
- [ ] リリースブランチ方針を確認する。`main` から直接タグを打つか、リリースブランチを使うかを決める。
- [ ] release workflow は既存の `cargo xtask desktop-package` / `.github/workflows/kukuri-release.yml` を拡張する方針で固定し、初回プレビューでは `tauri-action` への全面移行を行わない。
- [ ] GitHub Actions の実行トリガーを整理する。
  - `push tags: v*` もまず draft release を作る。
  - `workflow_dispatch` は tag input を必須にし、既定で draft release を作る。
  - どちらの経路でも release tag の checkout と version consistency gate を通す。
- [ ] `cargo xtask doctor` または専用のリリース確認コマンドに、バージョン同期チェックを追加する。
- [ ] ワークフロー確定後に `docs/runbooks/release.md` を追加する。

完了条件:

- 正となるリリースチェックリストが 1 つ存在する。
- バージョン/チャンネル規約が文書化されている。
- リリース候補ビルドを再現可能なコマンドで作成できる。

### フェーズ 1: アップデーター基盤

目的: インストール済みプレビュービルドが、手動再インストールなしで更新できるようにします。

作業:

- [ ] `tauri-plugin-updater` を `apps/desktop/src-tauri/Cargo.toml` に追加する。
- [ ] `@tauri-apps/plugin-updater` を `apps/desktop/package.json` に追加する。
- [ ] Tauri 起動時に updater plugin を登録する。
- [ ] updater の公開鍵と endpoint を `tauri.conf.json` またはリリース用 override 設定に追加する。
- [ ] Windows bundle で更新用成果物を作成する設定を有効にする。
- [ ] Windows の更新用成果物は `bundle.createUpdaterArtifacts: true` を使い、NSIS installer と updater bundle / signature の両方を release asset に含める。
- [ ] 更新マニフェスト名を決める。
  - プレビューチャンネルは `latest-preview.json`。
  - 安定版チャンネル用に `latest.json` を予約する。
- [ ] static updater manifest の必須項目を検証する。
  - `version`
  - `platforms.windows-x86_64.url`
  - `platforms.windows-x86_64.signature`
  - `signature` は `.sig` への URL ではなく、生成された `.sig` ファイルの内容を埋め込む。
- [ ] フロントエンド API 層に更新状態型を追加する。
  - `idle`
  - `checking`
  - `up_to_date`
  - `available`
  - `downloading`
  - `ready_to_restart`
  - `failed`
- [ ] Settings / About に「更新を確認」操作を追加する。
- [ ] 更新がある場合の非ブロッキングな更新バナーを追加する。
- [ ] インストール準備完了後の再起動プロンプトを追加する。
- [ ] ネットワーク失敗、マニフェスト取得失敗、署名検証失敗、インストール失敗を区別したエラー文を追加する。

完了条件:

- ローカルにインストールした旧ビルドが、テスト用マニフェストから新しいビルドを発見できる。
- 署名不一致の更新はインストールを拒否する。
- オフライン時の更新確認が安全に失敗表示される。
- 更新状態はローカルアクティビティ通知 inbox から独立している。

### フェーズ 2: リリースワークフローと署名

目的: GitHub Releases が、新規インストール用とアップデート用の両方に必要な成果物を生成します。

作業:

- [ ] `.github/workflows/kukuri-release.yml` を次の job 境界に分ける。
  - `validate-release-inputs`: tag/ref/channel/version consistency を検証する。
  - `linux-verify`: fast/nightly と同等以上の Linux gate を通す。
  - `windows-package`: clean な GitHub-hosted Windows runner で `cargo xtask desktop-package` を実行する。
  - `release-assets`: updater manifest、checksum、release note 下書き、可能なら provenance を生成する。
  - `publish-draft`: 既定では draft GitHub Release を作成する。
- [ ] `validate-release-inputs` で次を fail-fast する。
  - tag が存在しない。
  - tag が `vX.Y.Z-preview.N` に一致しない。
  - tag の version と `Cargo.toml` / `apps/desktop/package.json` / `apps/desktop/src-tauri/tauri.conf.json` の version が一致しない。
  - `workflow_dispatch` の tag input と checkout 対象 ref が一致しない。
- [ ] `windows-package` は次を公開する。
  - NSIS インストーラー。
  - 更新用バンドル。
  - `.sig` ファイル。
  - 必要に応じて通常 installer と updater bundle を区別できる artifact 名。
- [ ] updater 秘密鍵を GitHub Actions secrets に保存する。
- [ ] updater build では `TAURI_SIGNING_PRIVATE_KEY` と、必要であれば `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` を GitHub Actions secrets から渡す。
- [ ] updater 公開鍵はリポジトリ設定に保持する。
- [ ] `release-assets` で次を生成する。
  - `latest-preview.json`
  - `SHA256SUMS.txt`
  - release asset 一覧
  - manual smoke checklist
- [ ] `latest-preview.json` は同じ GitHub Release 内の変更されない asset URL を参照する。
- [ ] `latest-preview.json` の `signature` は `.sig` ファイルの内容を埋め込み、`.sig` の URL だけを入れない。
- [ ] マニフェストが存在しない配布物を参照している場合、または `.sig` の読み込み/埋め込みに失敗した場合、リリースワークフローを失敗させる検証を追加する。
- [ ] `SHA256SUMS.txt` は通常 installer、updater bundle、manifest を含める。
- [ ] GitHub Actions artifact attestation が利用できる場合、Windows 配布物と updater bundle に attestation を付ける。
- [ ] 証明書が利用可能になったら Windows コード署名 step を追加する。
- [ ] コード署名が初回プレビューに間に合わない場合、リリースノートに明示的な注意書きを追加し、SmartScreen の手動確認を行う。
- [ ] 成果物保持期間と成果物名で、preview / stable を区別できるようにする。
- [ ] draft release 作成後の手動 smoke が終わるまで、公開 release へ昇格しない。

完了条件:

- リリースワークフローの出力だけで、新規インストールと更新インストールの両方が成立する。
- 生成されたマニフェストが、同じリリース内の変更されない配布物 URL を参照している。
- 手動実行で、即時公開ではなく draft release を作成できる。
- draft release asset を差し替えずに、manual smoke 後にそのまま公開へ昇格できる。
- 証明書がない場合でも、未署名プレビューであること、SmartScreen warning が想定内であること、回避手順が release note に明記されている。

### フェーズ 3: アップデート E2E とデータ安全性

目的: アップデート後もユーザーデータが維持され、失敗時にもユーザーが詰まらないようにします。

作業:

- [ ] `v0.1.0-preview.1` と `v0.1.0-preview.2` のテスト成果物を作る。
- [ ] 手動または自動のアップデートシナリオを作る。
  - 旧ビルドをインストールする。
  - identity を作成する。
  - Community Node が ready になるまで待つ。
  - starter topic を追加する。
  - 投稿/返信する。
  - private channel を作成または参加する。
  - ローカル通知を受信または作成する。
  - 新ビルドへ更新する。
  - すべての状態が残っていることを確認する。
- [ ] 少なくとも 1 つの旧版 DB fixture を追加する。
- [ ] 再インストール時にデータを保持するか削除するかを文書化する。
- [ ] keyring fallback と file fallback のリスクを文書化する。
- [ ] マイグレーション失敗または DB open 失敗時のユーザー向け起動エラーを追加する。

完了条件:

- アップデート後も identity、DB、Iroh data、Community Node 設定、private channel capability、通知 inbox が保持される。
- 失敗したアップデートを再試行できる。
- マイグレーション失敗時に、空白画面ではなく実行可能な診断情報が表示される。

### フェーズ 4: 診断レポートとフィードバック導線

目的: プレビュー利用者が、秘密情報を漏らさずに有用なフィードバックを送れるようにします。

作業:

- [ ] Settings に診断レポートのコピー/書き出し操作を追加する。
- [ ] 診断レポートに次を含める。
  - アプリバージョン。
  - リリースチャンネル。
  - OS とアーキテクチャ。
  - 同期状態。
  - discovery mode。
  - Community Node の session phase / last error / retry-after。
  - active path と peer 数。
  - subscribed topic 数。
  - 通知未読数。
  - 直近の秘密情報を含まないエラーメッセージ。
  - 更新状態と直近の更新エラー。
- [ ] 次を除外または秘匿する。
  - secret key。
  - 認証 token。
  - private channel capability secret。
  - invite/share token。
  - DM の本文。
  - ユーザー名などを含むローカル DB path。ただし、ユーザーが詳細レポートを明示選択した場合は別扱いにできる。
- [ ] GitHub フィードバック URL またはテンプレート付きコピーを追加する。
- [ ] `docs/runbooks/mvp-user-quickstart.md` に、診断レポートを添える手順を追加する。
- [ ] `docs/runbooks/mvp-troubleshooting.md` に、更新と診断レポートの節を追加する。

完了条件:

- 技術に詳しくないユーザーでも、1 分以内に有用な不具合報告を作れる。
- 既定の診断レポートは、公開 issue に貼っても安全な内容になっている。

### フェーズ 5: 本番用セキュリティ強化

目的: プレビュービルドのセキュリティ姿勢を明確にし、範囲を絞ります。

作業:

- [ ] リリース設定の `csp: null` を本番用 CSP に置き換える。
- [ ] Tauri capability を確認し、必要な権限だけが有効になっていることを確認する。
- [ ] deep link parsing を監査し、未対応 scheme や壊れた token を拒否する。
- [ ] 更新 endpoint が HTTPS であることを確認する。
- [ ] すべての更新インストールで updater 署名検証を必須にする。
- [ ] サードパーティライセンス通知の生成を追加する。
- [ ] README またはアプリ内 settings に、プライバシーとデータ保存の説明を追加する。

完了条件:

- リリース設定は開発設定より明示的に厳しくなっている。
- deep link 処理が、壊れたデータや想定外のデータを黙って取り込まない。
- ユーザーが、何が端末に保存され、何が端末外へ出るのかを理解できる。

### フェーズ 6: OS 通知機能

目的: ローカル通知 inbox の意味を変えずに、OS 通知を利用できるようにします。

範囲:

- この機能は `NotificationRow` の永続化とは別です。
- 既存のローカル通知 inbox は、プロダクト内アクティビティ履歴の永続的な正本として維持します。
- OS 通知は、ユーザーの注意を促すための best effort な表示面です。

作業:

- [ ] desktop 向け OS 通知 plugin/dependency を追加する。
- [ ] 通知設定を追加する。
  - 全体の有効/無効。
  - ダイレクトメッセージ。
  - メンション/返信。
  - 必要であれば、フォロー/リポスト。
  - 簡単に入れられる範囲で静音モード。
  - プレビュー本文の表示/非表示。
- [ ] 権限要求フローを追加する。
- [ ] 配信方針を追加する。
  - 自分が作成したイベントでは通知しない。
  - 関連する pane がフォーカス中なら、設定で許可されていない限り通知しない。
  - プレビュー本文が無効なら、private DM の本文を OS 通知に含めない。
  - notification id または source event id で重複排除する。
- [ ] OS 通知クリック時の遷移先を決める。
  - ダイレクトメッセージは DM pane を開く。
  - 返信/メンション/リポストは thread または topic を開く。
  - フォロー通知は author pane を開く。
- [ ] 設定と重複排除方針のテストを追加する。
- [ ] 権限許可/拒否、フォアグラウンド/バックグラウンド、可能な範囲でアプリ終了時の手動 QA を追加する。

完了条件:

- OS 通知を無効にしても、ローカル通知 inbox には影響しない。
- ローカル通知 inbox の既読化やクリア操作が、OS 通知の権限設定を暗黙に変更しない。
- OS 通知クリック時の遷移は、可能な限り既存の通知クリック時遷移と一致する。

### フェーズ 7: 最終リリース候補確認

目的: 1 つのタグ付き候補をプレビュー公開可能と判断できる状態にします。

作業:

- [ ] リリースワークフローを draft mode で実行する。
- [ ] clean な Windows 10 / Windows 11 環境に draft 配布物をインストールする。
- [ ] happy path を完了する。
  - 起動する。
  - Community Node が ready になる。
  - starter topic を開く。
  - public post を行う。
  - reply/thread を確認する。
  - private channel を確認する。
  - テスト peer が使える場合は DM を確認する。
  - 通知 inbox を確認する。
  - 診断レポートを書き出す。
- [ ] 前回 RC からのアップデート経路を完了する。
- [ ] リリースノートに次が含まれることを確認する。
  - プレビュー範囲。
  - 既知の制限。
  - 更新挙動。
  - データ保存/プライバシーに関する注意。
  - フィードバック手順。
  - トラブルシューティングへのリンク。
- [ ] 最終 smoke 後にリリースを公開する。

完了条件:

- draft release の成果物を差し替えずに、そのまま公開へ昇格できる。
- 既知の制限が、ユーザーがインストーラーを入手する前に読める。

## 検証マトリクス

| 経路 | 確認コマンドまたは確認内容 |
| --- | --- |
| workspace static check | `cargo xtask check` |
| Rust tests | `cargo xtask rust-test` |
| desktop lint/typecheck | `cargo xtask desktop-lint` |
| desktop unit tests | `cargo xtask desktop-test` |
| Storybook build | `cargo xtask desktop-storybook` |
| browser UI tests | `cargo xtask desktop-browser-test` |
| Tauri compile check | `cargo xtask tauri-check` |
| Windows package | Windows 上で `cargo xtask desktop-package` |
| release input gate | tag/ref/channel/version consistency を release workflow で検証する |
| release asset manifest | `latest-preview.json` が同一 release asset URL と `.sig` 内容を参照していることを検証する |
| release checksum | `SHA256SUMS.txt` が installer / updater bundle / manifest を含むことを確認する |
| draft release smoke | draft release asset を差し替えずに Windows 10 / Windows 11 で install/update を確認する |
| artifact provenance | attestation を使う場合、`gh attestation verify` または同等手段で配布物の出所を確認する |
| smoke scenario | `cargo xtask e2e-smoke` |
| Community Node connectivity | `cargo xtask scenario community_node_public_connectivity` |
| updater test | 旧ビルドをインストールし、新ビルドへ更新し、データ保持を確認する |
| diagnostics test | 秘匿情報除去済みレポートを書き出し、秘密情報が含まれないことを確認する |
| OS notification test | 権限許可/拒否、フォアグラウンド/バックグラウンド、クリック時遷移を確認する |

## リリースチェックリスト

- [ ] `README.ja.md` と `README.md` が、プレビュー範囲と Windows インストーラー導線を説明している。
- [ ] `docs/runbooks/mvp-user-quickstart.md` に、更新確認と診断付きフィードバック手順がある。
- [ ] `docs/runbooks/mvp-troubleshooting.md` に、アップデーター、インストール失敗、更新失敗の状態説明がある。
- [ ] `docs/runbooks/release.md` が存在し、ワークフローと一致している。
- [ ] リリースワークフローが、tag/ref/channel/version consistency gate を通したうえで draft release を作成できる。
- [ ] draft release に、インストーラー、更新用ファイル、署名、チェックサム、マニフェスト、リリースノートが含まれている。
- [ ] draft release の成果物は GitHub-hosted runner で生成されたもので、手元ビルドとの差し替えを行っていない。
- [ ] `latest-preview.json` の `signature` は `.sig` URL ではなく `.sig` 内容である。
- [ ] `SHA256SUMS.txt` が release asset と一致している。
- [ ] artifact attestation を使う場合、検証手順が release note または runbook にある。
- [ ] インストーラーが署名済みである。未署名の場合は、未署名プレビューであるリスクが明記されている。
- [ ] updater マニフェストが、同じバージョン/チャンネルの配布物を参照している。
- [ ] 正しい署名と不正な署名の両方で、更新署名検証を確認している。
- [ ] 新規インストールの happy path を手動確認している。
- [ ] アップデートの happy path を手動確認している。
- [ ] 再インストール時の挙動を手動確認している。
- [ ] 診断レポートの書き出しと秘匿処理を手動確認している。
- [ ] 既存のローカル通知 inbox が、アクティビティ通知シナリオで引き続き通る。
- [ ] OS 通知設定が、ローカル通知 inbox の挙動を変更しない。
- [ ] プライバシー/データ保存説明が、初回利用前または初回利用中に読める。
- [ ] 既知の制限がリリースノートに列挙されている。

## 未決事項

- 初回プレビューに Windows コード署名を必須とするか、内部/ビルダー向けプレビューとして未署名のまま明示的な警告付きで配るか。
- updater マニフェストを GitHub Release asset のみで配るか、プロジェクト所有の安定 URL にもミラーするか。
- 診断レポートは初回プレビューではクリップボードのみでよいか、ZIP または text file 書き出しも行うか。
- OS 通知を初回公開プレビューに含めるか、アップデーターと診断レポートの直後に入れるか。
- 更新確認を起動時に自動実行するか、一定間隔で実行するか、初回プレビューでは手動確認のみにするか。
- artifact attestation を初回プレビューの必須条件にするか、P1 の provenance 強化として扱うか。
- 公開昇格を release workflow 内の environment approval にするか、GitHub Release UI で draft を手動 publish する運用にするか。

## 決定済み方針

- 初回プレビューでは、既存の `cargo xtask desktop-package` と `.github/workflows/kukuri-release.yml` を拡張する。`tauri-action` への全面移行は行わない。
- `latest-preview.json` は CI で生成し、GitHub Release asset として添付する。リリースメタデータ用ブランチへの commit は初回プレビューでは行わない。
- 初回プレビューの updater channel は preview のみとし、stable 用 `latest.json` は予約に留める。
- 手動実行の release workflow は draft release を作り、draft asset の manual smoke 後に公開する。

## このマイルストーンで扱わないこと

- 一般公開リリース。
- macOS パッケージングと notarization。
- Linux バイナリ配布。
- 静的 GitHub マニフェストで不足が出ない限り、動的な段階的配布サーバー。
- 通知のクロスデバイス同期。
- プッシュ通知サービス。
- 必須 telemetry。
- 完全な moderation tooling。

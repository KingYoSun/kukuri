# ドキュメント日付表記ガイドラインと棚卸し（最終更新日: 2025年10月22日）

## 背景
- タスク: ドキュメントの日付表記を `YYYY年MM月DD日` に統一するためのルール整理と主要ドキュメントの棚卸し。
- 実施日: 2025年10月22日
- 担当: Codex

## 日付表記ルール
- 基本形: `YYYY年MM月DD日`（例: `2025年10月22日`）。月と日も2桁でゼロ埋めする。
- メタ情報欄（`作成日` / `最終更新日` / `最終確認日` など）は同形式で統一し、必要に応じて文脈（担当者、更新理由）を後続テキストで補足する。
- 文中で日付を示す場合も同形式を用いる。タイムラインや更新履歴の箇条書きでもゼロ埋めを維持する。
- コマンドやスクリプト例では `date "+%Y年%m月%d日"`（Linux/macOS）あるいは `Get-Date -Format 'yyyy年MM月dd日'`（PowerShell）で生成する値を貼り付ける。
- 既存ドキュメントの差分確認には次のような正規表現を活用し、誤った表記（例: `YYYY年M月D日` のようにゼロ埋めされていない形）を検出する:  
  - 作業例: `grep -R "20[0-9][0-9]年[1-9]月" docs` / `grep -R "月[1-9]日" docs`
- ファイル名内のハイフン区切り（例: `2025-08-16`）は運用履歴上の識別子としてそのまま維持してよいが、本文内では統一形式に変換する。

## 主要ドキュメント棚卸し（2025年10月22日時点）
| カテゴリ | ドキュメント | チェック対象 | 表記状況 |
| --- | --- | --- | --- |
| プロジェクト概要 | `docs/SUMMARY.md` | 最終更新、更新履歴（計36件） | 対応済み（2025年10月22日修正、全項目ゼロ埋め済み） |
| プロジェクト概要 | `docs/01_project/design_doc.md` | 最終更新日 | 対応済み（2025年10月22日修正） |
| プロジェクト概要 | `docs/01_project/requirements.md` | 作成日、最終更新日 | 対応済み（2025年10月22日修正） |
| プロジェクト概要 | `docs/01_project/roadmap.md` | 作成日、最終更新 | 問題なし |
| プロジェクト概要 | `docs/01_project/refactoring_plan_2025-08-08_v3.md` | 作成日、改訂履歴 | 対応済み（2025年10月22日修正） |
| プロジェクト概要 | `docs/01_project/setup_guide.md` | 作成日、最終更新 | 対応済み（2025年10月22日修正） |
| プロジェクト概要 | `docs/01_project/windows_setup_guide.md` | 日付メタ情報 | 日付情報なし（必要なら追記検討） |
| ActiveContext | `docs/01_project/activeContext/summary.md` | 最終確認日、補足 | 問題なし |
| ActiveContext | `docs/01_project/activeContext/iroh-native-dht-plan.md` | 進捗・最終更新 | 問題なし |
| ActiveContext | `docs/01_project/activeContext/tauri_app_experience_design.md` | 作成日、最終更新 | 対応済み（2025年10月22日修正） |
| ActiveContext | `docs/01_project/activeContext/tauri_app_implementation_plan.md` | 作成日、進捗履歴 | 対応済み（2025年10月22日修正、5箇所ゼロ埋め） |
| ActiveContext | `docs/01_project/activeContext/tasks/README.md` | 最終更新 | 問題なし |
| ActiveContext | `docs/01_project/activeContext/tasks/priority/critical.md` | 最終更新 | 問題なし |
| ActiveContext | `docs/01_project/activeContext/tasks/status/in_progress.md` | 最終更新、履歴 | 問題なし |
| アーキテクチャ | `docs/02_architecture/system_design.md` | 作成日、最終更新、履歴 | 対応済み（2025年10月22日修正） |
| アーキテクチャ | `docs/02_architecture/project_structure.md` | 作成日、最終更新 | 対応済み（2025年10月22日修正） |
| アーキテクチャ | `docs/02_architecture/dht_discovery_architecture.md` | 作成日、最終更新 | 問題なし |
| アーキテクチャ | `docs/02_architecture/iroh_gossip_review.md` | 作成日 | 対応済み（2025年10月22日修正） |
| 実装ガイド | `docs/03_implementation/summary.md` | 最終更新 | 対応済み（2025年10月22日修正） |
| 実装ガイド | `docs/03_implementation/dht_integration_guide.md` | 作成日、最終更新 | 問題なし |
| 実装ガイド | `docs/03_implementation/implementation_plan.md` | 作成日、フェーズ履歴 | 対応済み（2025年10月22日修正、13箇所ゼロ埋め） |
| 実装ガイド | `docs/03_implementation/testing_guide.md` | 日付メタ情報 | 日付情報なし |
| 実装ガイド | `docs/03_implementation/docker_test_environment.md` | 作成日、最終更新 | 対応済み（2025年10月22日修正） |
| 実装ガイド | `docs/03_implementation/windows_test_docker_runbook.md` | 作成日、最終更新 | 問題なし |
| 実装ガイド | `docs/03_implementation/p2p_mainline_runbook.md` | 作成日、最終更新 | 問題なし |
| 実装ガイド | `docs/03_implementation/error_handling_guidelines.md` | 最終更新 | 問題なし |
| タスクアーカイブ | `docs/01_project/activeContext/tasks/completed/**/*.md` | 完了タスク記録 | 対応済み（2025年10月22日、一括スクリプトで修正） |
| タスクアーカイブ | `docs/01_project/activeContext/tasks/context/**/*.md` | ブロッカー・決定事項 | 対応済み（2025年10月22日、一括スクリプトで修正） |
| 進捗レポート | `docs/01_project/progressReports/*.md` | 作成日、更新履歴 | 対応済み（2025年10月22日、一括スクリプトで修正） |
| 実装ドキュメント補助 | `docs/03_implementation/*`（上記以外） | 作成日、更新履歴 | 対応済み（2025年10月22日、一括スクリプトで修正） |

## 推奨アクション
- 主要ドキュメントおよびアーカイブ/進捗レポート群は2025年10月22日にゼロ埋め対応済み。今後更新する際も同形式を維持し、変更が発生した場合は本棚卸し表を更新すること。
- 日付メタ情報が存在しないドキュメントについては、必要性をオーナーに確認し、定常的に追跡する場合は `作成日` / `最終更新日` を追加する。
- 定期棚卸しを月次で実施し、`grep` / `python` スクリプトで自動抽出した結果を反映する。

## 参考スクリプト
- CI チェック: `python scripts/check_date_format.py`  
- 自動修正: `python scripts/check_date_format.py --fix`

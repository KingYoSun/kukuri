# SQLx ベストプラクティス

**作成日**: 2025年8月13日

## 概要

このドキュメントでは、kukuriプロジェクトにおけるSQLxの使用に関するベストプラクティスをまとめています。特に、オフラインモードでのコンパイルとDocker環境でのテスト実行に関する重要な情報を記載しています。

## SQLxオフラインモードの設定

### 1. 初期設定

SQLxのquery!マクロを使用する場合、開発環境でデータベースを準備する必要があります：

```bash
# データベースの作成
cd kukuri-tauri/src-tauri
DATABASE_URL="sqlite:data/kukuri.db" sqlx database create

# マイグレーションの実行
DATABASE_URL="sqlite:data/kukuri.db" sqlx migrate run
```

### 2. クエリキャッシュの生成

SQLxはコンパイル時にデータベースへの接続を必要としますが、オフラインモードを使用することで、事前に生成したキャッシュを使用できます：

```bash
# .sqlxディレクトリにクエリキャッシュを生成
cd kukuri-tauri/src-tauri
DATABASE_URL="sqlite:data/kukuri.db" cargo sqlx prepare --workspace
```

このコマンドにより、`.sqlx`ディレクトリに各クエリのキャッシュファイル（`query-{hash}.json`）が生成されます。

### 3. 重要な注意事項

- **クエリ変更時**: データベーススキーマやquery!マクロ内のクエリを変更した場合、必ず`cargo sqlx prepare`を再実行する
- **バージョン管理**: `.sqlx`ディレクトリは必ずGitにコミットする（チーム全体で共有するため）
- **Docker環境**: Dockerfile内で`.sqlx`ディレクトリをコピーする必要がある

## Docker環境での設定

### Dockerfile.testの例

```dockerfile
# SQLxキャッシュをコピー（重要）
COPY kukuri-tauri/src-tauri/.sqlx ./kukuri-tauri/src-tauri/.sqlx
```

### Docker環境でのテスト実行

```bash
# Windows環境
.\scripts\test-docker.ps1 rust

# キャッシュの問題が発生した場合
.\scripts\test-docker.ps1 clean  # イメージをクリーンアップ
.\scripts\test-docker.ps1 rust   # 再ビルドして実行
```

## トラブルシューティング

### エラー: "set DATABASE_URL to use query macros online"

**原因**: `.sqlx`ディレクトリが存在しない、またはキャッシュが古い

**解決方法**:
1. `cargo sqlx prepare`を実行してキャッシュを更新
2. `.sqlx`ディレクトリがGitにコミットされているか確認
3. Dockerイメージを再ビルド

### エラー: "SQLX_OFFLINE=true but there is no cached data"

**原因**: 新しいクエリがキャッシュに含まれていない

**解決方法**:
```bash
DATABASE_URL="sqlite:data/kukuri.db" cargo sqlx prepare --workspace
git add .sqlx
git commit -m "Update SQLx query cache"
```

## 開発フロー

1. **新しいクエリを追加した場合**:
   ```rust
   sqlx::query!(
       "INSERT INTO new_table (column1, column2) VALUES (?1, ?2)",
       value1,
       value2
   )
   ```

2. **キャッシュを更新**:
   ```bash
   DATABASE_URL="sqlite:data/kukuri.db" cargo sqlx prepare --workspace
   ```

3. **変更をコミット**:
   ```bash
   git add .sqlx src/
   git commit -m "Add new query and update SQLx cache"
```

4. **テスト実行**:
   ```bash
   # ローカル
   cargo test
   
   # Docker環境
   .\scripts\test-docker.ps1 rust
   ```

## フェーズ3/4リファクタリング後の `.sqlx` 更新フロー

Phase 3/4 で `sqlite_repository` がモジュール分割され、マッパーが `application/shared/mappers` に集約されました。依存関係が広範囲に及ぶため、`.sqlx` を更新する際は以下のチェックリストに従います。

1. **分割後の構成を確認**  
   - `kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository/` 配下が `posts.rs` などに分割されていること。  
   - マッパーが `application/shared/mappers` から再利用されている場合、関連する構造体定義の変更有無を確認。

2. **オフラインキャッシュを再生成**  
   ```bash
   cd kukuri-tauri/src-tauri
   setx DATABASE_URL "sqlite:data/kukuri.db"  # PowerShell の場合は $env:DATABASE_URL を使用
   $env:DATABASE_URL="sqlite:data/kukuri.db" cargo sqlx prepare
   ```
   - Windows では PowerShell で環境変数を設定したうえで実行する。  
   - `--workspace` は必要なときのみ指定し、想定外のクレートまで再生成しないよう注意。

3. **成果物をバージョン管理する**  
   - `.sqlx/` 配下のタイムスタンプと差分を確認し、不要なファイル削除が発生していないかチェック。  
   - SQL を更新していないにも関わらず差分が出た場合はコミット前に原因を調査。

4. **検証と記録**  
   - `cargo fmt && cargo clippy -D warnings && cargo test` を通過させる。  
   - `tasks/status/in_progress.md` に再生成の実施日と対象クエリをメモし、レビュー時に参照できる状態を維持する。

### リスク評価と後方互換性チェック

- **生成キャッシュとコードの乖離**  
  - マッパーや SQL をリファクタリングした場合、`.sqlx` を更新しないままだと CI で `SQLX_OFFLINE=true but there is no cached data` が発生する。  
  - `cargo sqlx prepare` 後は必ずローカルテストを実行し、生成物とコードが整合しているかを確認する。

- **ローカルストレージとの整合性**  
  - Phase 4 で Zustand 永続化ロジックが `withPersist` と共通 `createMapAwareStorage` に統一された。`.sqlx` 再生成を含むリリースでは、ローカルストレージキー（`persistKeys.*`）のリネーム有無を併せて確認し、必要に応じて移行手順をリリースノートへ記載する。  
  - 既存データを破棄する必要がある場合は、UI 初回起動時のクリア処理やバックアップ取得手順を説明する。

- **CI 環境でのオフライン失敗**  
  - `.sqlx` が最新でない状態で CI を流すと、フェーズ分割後のモジュールが参照するクエリキャッシュが欠落する。  
  - PR では `cargo sqlx prepare` 実行結果を明示し、レビュー時に `.sqlx` の差分が期待通りか確認するチェックボックスを活用する。

## ベストプラクティス

### 1. query!マクロ vs query_as

- **query!マクロ**: コンパイル時の型安全性が高い、オフラインモードのセットアップが必要
- **query_as**: 実行時の型チェック、セットアップが簡単

プロジェクトでは型安全性を重視してquery!マクロを使用することを推奨します。

### 2. CI/CD環境

CI/CDパイプラインでは、`.sqlx`ディレクトリがリポジトリに含まれていることを前提としています：

```yaml
# GitHub Actions例
- name: Run tests
  run: |
    cargo test --all-features
  env:
    SQLX_OFFLINE: true
```

### 3. チーム開発

- `.sqlx`ディレクトリは必ずバージョン管理に含める
- データベーススキーマ変更時は、PRに`.sqlx`の更新を含める
- レビュー時にクエリキャッシュの更新を確認

## まとめ

SQLxのオフラインモードを適切に設定することで、以下のメリットが得られます：

- **高速なビルド**: データベース接続が不要
- **CI/CD対応**: データベースサーバーなしでテスト実行可能
- **型安全性**: コンパイル時のクエリ検証

重要なのは、クエリを変更したら必ず`cargo sqlx prepare`を実行し、`.sqlx`ディレクトリをコミットすることです。

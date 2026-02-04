# GitHub Actions Desktop E2E 失敗対応（tauri-driver ポート競合）

- scripts/docker/run-desktop-e2e.sh で空きポート自動選定を追加し、TAURI_DRIVER_PORT 未指定時に 4700-5200 を探索して使用
- Desktop E2E（Community Node, Docker）を ./scripts/test-docker.ps1 e2e-community-node で再実行し成功
- gh act で format-check / native-test-linux を実行して成功（git clone の some refs were not updated と React act/useRouter 警告は既知）

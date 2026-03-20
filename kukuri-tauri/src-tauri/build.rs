fn main() {
    tauri_build::build();
    // Windows: 二重の `winres` / `WindowsResource::compile()` を足さないこと（`tauri-build` が既に
    // `resource.lib` をリンクする）。詳細は `docs/03_implementation/bugfix_windows_msvc_duplicate_version_resource.md`。
}

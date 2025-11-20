/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly TAURI_ENV_DEBUG?: 'true' | 'false';
  readonly VITE_ENABLE_E2E?: 'true';
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly TAURI_ENV_DEBUG?: 'true' | 'false';
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

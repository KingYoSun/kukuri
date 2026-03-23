/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_KUKURI_DESKTOP_MOCK?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

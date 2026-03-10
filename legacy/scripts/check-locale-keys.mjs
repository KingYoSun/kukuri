#!/usr/bin/env node
import { promises as fs } from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");
const localesDir = path.join(repoRoot, "kukuri-tauri", "src", "locales");

const localeFiles = [
  { locale: "ja", file: "ja.json" },
  { locale: "en", file: "en.json" },
  { locale: "zh-CN", file: "zh-CN.json" },
];

function flattenKeys(value, prefix = "") {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return prefix ? [prefix] : [];
  }

  const entries = Object.entries(value);
  if (entries.length === 0) {
    return prefix ? [prefix] : [];
  }

  return entries.flatMap(([key, child]) => {
    const childPrefix = prefix ? `${prefix}.${key}` : key;
    return flattenKeys(child, childPrefix);
  });
}

async function readLocaleKeys(file) {
  const filePath = path.join(localesDir, file);
  const raw = await fs.readFile(filePath, "utf-8");
  const json = JSON.parse(raw);
  return new Set(flattenKeys(json));
}

async function main() {
  const keySets = new Map();

  await Promise.all(
    localeFiles.map(async ({ locale, file }) => {
      keySets.set(locale, await readLocaleKeys(file));
    }),
  );

  const allKeys = new Set();
  for (const keySet of keySets.values()) {
    for (const key of keySet) {
      allKeys.add(key);
    }
  }

  let hasDrift = false;
  for (const { locale } of localeFiles) {
    const keySet = keySets.get(locale);
    const missing = Array.from(allKeys)
      .filter((key) => !keySet.has(key))
      .sort();
    if (missing.length > 0) {
      hasDrift = true;
      console.error(`[${locale}] missing keys (${missing.length})`);
      for (const key of missing) {
        console.error(`  - ${key}`);
      }
    }
  }

  if (hasDrift) {
    process.exit(1);
  }

  console.log(
    `Locale keys are aligned across ${localeFiles.map(({ locale }) => locale).join(", ")} (${allKeys.size} keys).`,
  );
}

main().catch((error) => {
  console.error("Failed to check locale keys:", error);
  process.exit(1);
});

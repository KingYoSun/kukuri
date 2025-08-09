import { invoke } from '@tauri-apps/api/core';

/**
 * 16進数の公開鍵をnpub（Bech32形式）に変換
 * @param pubkey 16進数の公開鍵
 * @returns npub形式の公開鍵
 */
export async function pubkeyToNpub(pubkey: string): Promise<string> {
  try {
    return await invoke('pubkey_to_npub', { pubkey });
  } catch (error) {
    console.error('Failed to convert pubkey to npub:', error);
    // エラー時はそのまま返す（fallback）
    return pubkey;
  }
}

/**
 * npub（Bech32形式）を16進数の公開鍵に変換
 * @param npub npub形式の公開鍵
 * @returns 16進数の公開鍵
 */
export async function npubToPubkey(npub: string): Promise<string> {
  try {
    return await invoke('npub_to_pubkey', { npub });
  } catch (error) {
    console.error('Failed to convert npub to pubkey:', error);
    // エラー時はそのまま返す（fallback）
    return npub;
  }
}

/**
 * 公開鍵がnpub形式かどうかを判定
 * @param key 公開鍵
 * @returns npub形式の場合true
 */
export function isNpubFormat(key: string): boolean {
  return key.startsWith('npub1');
}

/**
 * 公開鍵が16進数形式かどうかを判定
 * @param key 公開鍵
 * @returns 16進数形式の場合true
 */
export function isHexFormat(key: string): boolean {
  return /^[0-9a-fA-F]{64}$/.test(key);
}

/**
 * 任意の形式の公開鍵をnpub形式に正規化
 * @param key 公開鍵（npubまたは16進数）
 * @returns npub形式の公開鍵
 */
export async function normalizeToNpub(key: string): Promise<string> {
  if (isNpubFormat(key)) {
    return key;
  }
  if (isHexFormat(key)) {
    return await pubkeyToNpub(key);
  }
  // 既にnpubでも16進数でもない場合はそのまま返す
  return key;
}

/**
 * 任意の形式の公開鍵を16進数形式に正規化
 * @param key 公開鍵（npubまたは16進数）
 * @returns 16進数形式の公開鍵
 */
export async function normalizeToHex(key: string): Promise<string> {
  if (isHexFormat(key)) {
    return key;
  }
  if (isNpubFormat(key)) {
    return await npubToPubkey(key);
  }
  // 既にnpubでも16進数でもない場合はそのまま返す
  return key;
}
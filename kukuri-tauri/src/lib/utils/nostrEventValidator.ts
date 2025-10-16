// 最小限のNIPフォーマット検証（フロント側・型/形式のみ）
// 厳密なID再計算や署名検証はバックエンドに委譲

import { bech32 } from '@scure/base';

const HEX = /^[0-9a-f]+$/i;
const WS_URL_REGEX = /^wss?:\/\/.+/i;
const MAX_TLV_RELAY_URLS = 16;
const MAX_TLV_RELAY_URL_LEN = 255;
const utf8Decoder = new TextDecoder('utf-8', { fatal: true });

export type ValidationResult = { ok: true } | { ok: false; reason: string };

export function validateNip01Json(ev: any): ValidationResult {
  if (!ev || typeof ev !== 'object') return { ok: false, reason: 'not an object' };
  if (typeof ev.id !== 'string' || ev.id.length !== 64 || !HEX.test(ev.id)) {
    return { ok: false, reason: 'invalid id (64 hex)' };
  }
  if (typeof ev.pubkey !== 'string' || ev.pubkey.length !== 64 || !HEX.test(ev.pubkey)) {
    return { ok: false, reason: 'invalid pubkey (64 hex)' };
  }
  if (typeof ev.kind !== 'number') return { ok: false, reason: 'invalid kind' };
  if (typeof ev.created_at !== 'number') return { ok: false, reason: 'invalid created_at' };
  if (
    !Array.isArray(ev.tags) ||
    !ev.tags.every((t: any) => Array.isArray(t) && t.every((s: any) => typeof s === 'string'))
  ) {
    return { ok: false, reason: 'invalid tags (string[][])' };
  }
  if (typeof ev.content !== 'string') return { ok: false, reason: 'invalid content' };
  if (typeof ev.sig !== 'string' || ev.sig.length !== 128 || !HEX.test(ev.sig)) {
    return { ok: false, reason: 'invalid sig (128 hex)' };
  }
  return { ok: true };
}

// UIのP2Pメッセージ（最小フィールド）の軽量検証
// shape: { id, author, content, timestamp, signature }
export function validateNip01LiteMessage(msg: any): ValidationResult {
  if (!msg || typeof msg !== 'object') return { ok: false, reason: 'not an object' };
  if (typeof msg.id !== 'string' || msg.id.length !== 64 || !HEX.test(msg.id)) {
    return { ok: false, reason: 'invalid id (64 hex)' };
  }
  if (typeof msg.author !== 'string' || msg.author.length !== 64 || !HEX.test(msg.author)) {
    return { ok: false, reason: 'invalid author (64 hex pubkey)' };
  }
  if (
    typeof msg.signature !== 'string' ||
    msg.signature.length !== 128 ||
    !HEX.test(msg.signature)
  ) {
    return { ok: false, reason: 'invalid signature (128 hex)' };
  }
  if (typeof msg.content !== 'string') return { ok: false, reason: 'invalid content' };
  if (typeof msg.timestamp !== 'number') return { ok: false, reason: 'invalid timestamp' };
  return { ok: true };
}

export function validateNip10Basic(ev: any): ValidationResult {
  if (!Array.isArray(ev?.tags)) return { ok: false, reason: 'no tags' };
  let root = 0,
    reply = 0;
  for (const tag of ev.tags as string[][]) {
    if (!Array.isArray(tag) || tag.length < 2) continue;
    const [t, id, relay, marker] = tag;
    if (t === 'e') {
      // 参照IDは64hex or bech32(note|nevent)
      const isHex = typeof id === 'string' && id.length === 64 && HEX.test(id);
      const isBech = typeof id === 'string' && isValidEventReference(id);
      if (!isHex && !isBech) return { ok: false, reason: 'invalid e tag id' };
      if (relay && typeof relay === 'string' && relay.length > 0 && !isWsUrl(relay)) {
        return { ok: false, reason: 'invalid e tag relay_url' };
      }
      if (marker === 'root') root++;
      if (marker === 'reply') reply++;
      if (marker && !['root', 'reply', 'mention'].includes(marker)) {
        return { ok: false, reason: 'invalid e tag marker' };
      }
    }
    if (t === 'p') {
      const isHex = typeof id === 'string' && id.length === 64 && HEX.test(id);
      const isBech = typeof id === 'string' && isValidPubkeyReference(id);
      if (!isHex && !isBech) return { ok: false, reason: 'invalid p tag pubkey' };
      if (relay && typeof relay === 'string' && relay.length > 0 && !isWsUrl(relay)) {
        return { ok: false, reason: 'invalid p tag relay_url' };
      }
    }
  }
  if (root > 1) return { ok: false, reason: 'multiple root markers' };
  if (reply > 1) return { ok: false, reason: 'multiple reply markers' };
  return { ok: true };
}

function isWsUrl(url: string): boolean {
  return WS_URL_REGEX.test(url);
}

function isValidEventReference(value: string): boolean {
  if (value.startsWith('note1')) {
    return isBech32Payload(value, 'note', 32);
  }
  if (value.startsWith('nevent1')) {
    return isValidNeventTlv(value);
  }
  return false;
}

function isValidPubkeyReference(value: string): boolean {
  if (value.startsWith('npub1')) {
    return isBech32Payload(value, 'npub', 32);
  }
  if (value.startsWith('nprofile1')) {
    return isValidNprofileTlv(value);
  }
  return false;
}

function isBech32Payload(bech: string, expectedHrp: string, expectedLen: number): boolean {
  const bytes = decodeBech32(bech, expectedHrp);
  return !!bytes && bytes.length === expectedLen;
}

function isValidNprofileTlv(bech: string): boolean {
  const bytes = decodeBech32(bech, 'nprofile');
  if (!bytes) return false;
  let hasPubkey = false;
  let relayCount = 0;
  const ok = parseTlv(bytes, (tag, value) => {
    if (tag === 0) {
      if (hasPubkey || value.length !== 32) return false;
      hasPubkey = true;
    } else if (tag === 1) {
      relayCount += 1;
      if (relayCount > MAX_TLV_RELAY_URLS || !validateRelayTlv(value)) return false;
    }
    return true;
  });
  return ok && hasPubkey;
}

function isValidNeventTlv(bech: string): boolean {
  const bytes = decodeBech32(bech, 'nevent');
  if (!bytes) return false;
  let hasEventId = false;
  let hasAuthor = false;
  let hasKind = false;
  let relayCount = 0;
  const ok = parseTlv(bytes, (tag, value) => {
    if (tag === 0) {
      if (hasEventId || value.length !== 32) return false;
      hasEventId = true;
    } else if (tag === 1) {
      relayCount += 1;
      if (relayCount > MAX_TLV_RELAY_URLS || !validateRelayTlv(value)) return false;
    } else if (tag === 2) {
      if (hasAuthor || value.length !== 32) return false;
      hasAuthor = true;
    } else if (tag === 3) {
      if (hasKind || value.length !== 4) return false;
      hasKind = true;
    }
    return true;
  });
  return ok && hasEventId;
}

function decodeBech32(value: string, expectedHrp: string): Uint8Array | null {
  try {
    const { prefix, words } = bech32.decode(value, 1023);
    if (prefix !== expectedHrp) return null;
    return Uint8Array.from(bech32.fromWords(words));
  } catch {
    return null;
  }
}

function parseTlv(
  bytes: Uint8Array,
  handler: (tag: number, value: Uint8Array) => boolean
): boolean {
  let i = 0;
  while (i + 2 <= bytes.length) {
    const tag = bytes[i];
    const len = bytes[i + 1];
    i += 2;
    if (i + len > bytes.length) return false;
    const value = bytes.subarray(i, i + len);
    if (!handler(tag, value)) return false;
    i += len;
  }
  return i === bytes.length;
}

function validateRelayTlv(value: Uint8Array): boolean {
  if (value.length > MAX_TLV_RELAY_URL_LEN) return false;
  if (value.length === 0) return true;
  let decoded: string;
  try {
    decoded = utf8Decoder.decode(value);
  } catch {
    return false;
  }
  if (!isAscii(decoded)) return false;
  return isWsUrl(decoded);
}

function isAscii(value: string): boolean {
  for (let i = 0; i < value.length; i++) {
    if (value.charCodeAt(i) > 0x7f) return false;
  }
  return true;
}

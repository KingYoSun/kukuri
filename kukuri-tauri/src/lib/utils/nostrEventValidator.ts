// 最小限のNIPフォーマット検証（フロント側・型/形式のみ）
// 厳密なID再計算や署名検証はバックエンドに委譲

const HEX = /^[0-9a-f]+$/i;

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
      const isBech = typeof id === 'string' && (id.startsWith('note1') || id.startsWith('nevent1'));
      if (!isHex && !isBech) return { ok: false, reason: 'invalid e tag id' };
      if (relay && typeof relay === 'string' && relay.length > 0 && !/^wss?:\/\//i.test(relay)) {
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
      const isBech =
        typeof id === 'string' && (id.startsWith('npub1') || id.startsWith('nprofile1'));
      if (!isHex && !isBech) return { ok: false, reason: 'invalid p tag pubkey' };
      if (relay && typeof relay === 'string' && relay.length > 0 && !/^wss?:\/\//i.test(relay)) {
        return { ok: false, reason: 'invalid p tag relay_url' };
      }
    }
  }
  if (root > 1) return { ok: false, reason: 'multiple root markers' };
  if (reply > 1) return { ok: false, reason: 'multiple reply markers' };
  return { ok: true };
}

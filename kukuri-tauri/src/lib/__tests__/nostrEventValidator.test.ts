import { describe, it, expect } from 'vitest';
import { bech32 } from '@scure/base';
import { validateNip01Json, validateNip10Basic } from '../utils/nostrEventValidator';

function hex(len: number) {
  return 'f'.repeat(len);
}

function hexToBytes(value: string): Uint8Array {
  const clean = value.toLowerCase();
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

function makeBech32(prefix: string, payload: Uint8Array): string {
  return bech32.encode(prefix, bech32.toWords(payload), 1023);
}

function makeNpub(hexPubkey: string): string {
  return makeBech32('npub', hexToBytes(hexPubkey));
}

function makeTlv(entries: Array<{ tag: number; value: Uint8Array }>): Uint8Array {
  const parts: number[] = [];
  for (const { tag, value } of entries) {
    parts.push(tag);
    parts.push(value.length);
    for (const byte of value) {
      parts.push(byte);
    }
  }
  return new Uint8Array(parts);
}

function makeNprofile(hexPubkey: string, relays: string[] = []): string {
  const entries: Array<{ tag: number; value: Uint8Array }> = [
    { tag: 0, value: hexToBytes(hexPubkey) },
  ];
  for (const relay of relays) {
    entries.push({ tag: 1, value: new TextEncoder().encode(relay) });
  }
  return makeBech32('nprofile', makeTlv(entries));
}

function makeNevent(
  hexId: string,
  options: { relays?: string[]; author?: string; kind?: number } = {},
): string {
  const entries: Array<{ tag: number; value: Uint8Array }> = [{ tag: 0, value: hexToBytes(hexId) }];
  for (const relay of options.relays ?? []) {
    entries.push({ tag: 1, value: new TextEncoder().encode(relay) });
  }
  if (options.author) {
    entries.push({ tag: 2, value: hexToBytes(options.author) });
  }
  if (typeof options.kind === 'number') {
    const kindBuffer = new Uint8Array(4);
    const view = new DataView(kindBuffer.buffer);
    view.setUint32(0, options.kind, false);
    entries.push({ tag: 3, value: kindBuffer });
  }
  return makeBech32('nevent', makeTlv(entries));
}

describe('nostrEventValidator', () => {
  it('validates NIP-01 shape', () => {
    const ev = {
      id: hex(64),
      pubkey: hex(64),
      created_at: Math.floor(Date.now() / 1000),
      kind: 1,
      tags: [
        ['t', 'test'],
        ['e', hex(64)],
      ],
      content: 'hello',
      sig: hex(128),
    };
    expect(validateNip01Json(ev).ok).toBe(true);
  });

  it('rejects invalid sig', () => {
    const ev = {
      id: hex(64),
      pubkey: hex(64),
      created_at: 1,
      kind: 1,
      tags: [],
      content: '',
      sig: 'zz',
    };
    expect(validateNip01Json(ev).ok).toBe(false);
  });

  it('validates NIP-10 basics', () => {
    const npub = makeNpub(hex(64));
    const nevent = makeNevent(hex(64), {
      relays: ['wss://relay.example'],
      author: hex(64),
      kind: 1,
    });
    const ev = {
      tags: [
        ['e', nevent, 'wss://relay.example', 'reply'],
        ['p', npub, 'ws://relay.example'],
      ],
    };
    expect(validateNip10Basic(ev).ok).toBe(true);
  });

  it('rejects bad relay_url marker', () => {
    const ev = { tags: [['e', hex(64), 'http://example', 'bad']] };
    expect(validateNip10Basic(ev).ok).toBe(false);
  });

  it('rejects invalid nevent tlv', () => {
    const badNevent = makeBech32(
      'nevent',
      makeTlv([{ tag: 1, value: new TextEncoder().encode('wss://relay.example') }]),
    );
    const ev = { tags: [['e', badNevent]] };
    expect(validateNip10Basic(ev).ok).toBe(false);
  });

  it('rejects nprofile with non-ws relay', () => {
    const nprofile = makeNprofile(hex(64), ['https://relay.example']);
    const ev = { tags: [['p', nprofile]] };
    expect(validateNip10Basic(ev).ok).toBe(false);
  });

  it('rejects nprofile with utf8 violation', () => {
    const invalidBytes = new Uint8Array([0xff, 0xfe]);
    const badProfile = makeBech32(
      'nprofile',
      makeTlv([
        { tag: 0, value: hexToBytes(hex(64)) },
        { tag: 1, value: invalidBytes },
      ]),
    );
    const ev = { tags: [['p', badProfile]] };
    expect(validateNip10Basic(ev).ok).toBe(false);
  });
});

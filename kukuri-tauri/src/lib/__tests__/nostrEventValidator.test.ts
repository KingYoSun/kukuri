import { describe, it, expect } from 'vitest';
import { validateNip01Json, validateNip10Basic } from '../utils/nostrEventValidator';

function hex(len: number) { return 'f'.repeat(len); }

describe('nostrEventValidator', () => {
  it('validates NIP-01 shape', () => {
    const ev = {
      id: hex(64),
      pubkey: hex(64),
      created_at: Math.floor(Date.now()/1000),
      kind: 1,
      tags: [['t','test'], ['e', hex(64)]],
      content: 'hello',
      sig: hex(128),
    };
    expect(validateNip01Json(ev).ok).toBe(true);
  });

  it('rejects invalid sig', () => {
    const ev = { id: hex(64), pubkey: hex(64), created_at: 1, kind: 1, tags: [], content: '', sig: 'zz' };
    expect(validateNip01Json(ev).ok).toBe(false);
  });

  it('validates NIP-10 basics', () => {
    const ev = { tags: [ ['e', hex(64), 'wss://relay.example', 'reply'], ['p', hex(64), 'ws://relay.example'] ] };
    expect(validateNip10Basic(ev).ok).toBe(true);
  });

  it('rejects bad relay_url marker', () => {
    const ev = { tags: [ ['e', hex(64), 'http://example', 'bad'] ] };
    expect(validateNip10Basic(ev).ok).toBe(false);
  });
});


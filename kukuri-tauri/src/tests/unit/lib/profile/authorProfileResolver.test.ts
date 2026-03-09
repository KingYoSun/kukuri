import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  clearResolvedAuthorProfileCache,
  resolveAuthorProfileWithRelayFallback,
} from '@/lib/profile/authorProfileResolver';
import { TauriApi } from '@/lib/api/tauri';
import { subscribeToUser } from '@/lib/api/nostr';

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getUserProfileByPubkey: vi.fn(),
    getUserProfile: vi.fn(),
  },
}));

vi.mock('@/lib/api/nostr', () => ({
  subscribeToUser: vi.fn(),
}));

vi.mock('@/lib/utils/nostr', () => ({
  isHexFormat: (value: string) => /^[0-9a-fA-F]{64}$/.test(value),
  isNpubFormat: (value: string) => value.startsWith('npub1'),
  npubToPubkey: vi.fn(async () => 'a'.repeat(64)),
  pubkeyToNpub: vi.fn(async (pubkey: string) => `npub1${pubkey.slice(0, 10)}`),
}));

const authorPubkey = 'a'.repeat(64);

describe('authorProfileResolver', () => {
  beforeEach(() => {
    clearResolvedAuthorProfileCache();
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.mocked(TauriApi.getUserProfileByPubkey).mockResolvedValue(null);
    vi.mocked(TauriApi.getUserProfile).mockResolvedValue(null);
    vi.mocked(subscribeToUser).mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.useRealTimers();
    clearResolvedAuthorProfileCache();
  });

  it('relay subscribe 後に永続 profile を再取得できれば解決結果を返す', async () => {
    vi.mocked(TauriApi.getUserProfileByPubkey).mockResolvedValueOnce(null).mockResolvedValueOnce({
      npub: 'npub1relayalice',
      pubkey: authorPubkey,
      name: 'relay-alice',
      display_name: 'Relay Alice',
      about: 'relay profile',
      picture:
        'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgwJ/l7hR9QAAAABJRU5ErkJggg==',
      banner: null,
      website: null,
      nip05: 'alice@example.com',
      is_profile_public: true,
      show_online_status: false,
    });

    const promise = resolveAuthorProfileWithRelayFallback(authorPubkey);
    await vi.runAllTimersAsync();
    const resolved = await promise;

    expect(subscribeToUser).toHaveBeenCalledWith(authorPubkey);
    expect(resolved?.displayName).toBe('Relay Alice');
    expect(resolved?.picture).toContain('data:image/png;base64,');
  });

  it('直近 miss 後は relay subscribe を連打しない', async () => {
    const firstAttempt = resolveAuthorProfileWithRelayFallback(authorPubkey);
    await vi.runAllTimersAsync();
    await expect(firstAttempt).resolves.toBeNull();

    vi.mocked(subscribeToUser).mockClear();
    vi.mocked(TauriApi.getUserProfileByPubkey).mockClear();

    const secondAttempt = await resolveAuthorProfileWithRelayFallback(authorPubkey);

    expect(secondAttempt).toBeNull();
    expect(subscribeToUser).not.toHaveBeenCalled();
    expect(TauriApi.getUserProfileByPubkey).not.toHaveBeenCalled();
  });
});

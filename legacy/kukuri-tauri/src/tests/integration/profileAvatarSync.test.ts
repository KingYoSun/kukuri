import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { TauriApi } from '@/lib/api/tauri';
import { invoke } from '@tauri-apps/api/core';
import { setupIntegrationTest, setMockResponse } from './setup';

describe('Profile avatar sync flow', () => {
  let cleanup: () => void;

  beforeEach(() => {
    cleanup = setupIntegrationTest();
    vi.mocked(invoke).mockClear();
  });

  afterEach(() => {
    cleanup();
  });

  it('invokes upload_profile_avatar with encrypted payload metadata', async () => {
    const mockResponse = {
      npub: 'npub1example',
      blob_hash: 'abc123',
      format: 'image/png',
      size_bytes: 3,
      access_level: 'contacts_only',
      share_ticket: 'ticket-1',
      doc_version: 2,
      updated_at: '2025-11-02T12:00:00Z',
      content_sha256: 'deadbeef',
    } as const;
    setMockResponse('upload_profile_avatar', mockResponse);

    const payload = Uint8Array.from([1, 2, 3]);
    const result = await TauriApi.uploadProfileAvatar({
      npub: 'npub1example',
      data: payload,
      format: 'image/png',
      accessLevel: 'contacts_only',
    });

    expect(result.blob_hash).toBe('abc123');
    expect(result.access_level).toBe('contacts_only');

    const calls = vi.mocked(invoke).mock.calls;
    expect(calls).toHaveLength(1);
    expect(calls[0][0]).toBe('upload_profile_avatar');
    expect(calls[0][1]).toEqual({
      request: {
        npub: 'npub1example',
        bytes: [1, 2, 3],
        format: 'image/png',
        access_level: 'contacts_only',
      },
    });
  });

  it('retrieves and decodes fetched avatar payload', async () => {
    const mockResponse = {
      npub: 'npub1example',
      blob_hash: 'abc123',
      format: 'image/png',
      size_bytes: 3,
      access_level: 'contacts_only',
      share_ticket: 'ticket-1',
      doc_version: 2,
      updated_at: '2025-11-02T12:00:00Z',
      content_sha256: 'deadbeef',
      data_base64: 'AQID',
    } as const;
    setMockResponse('fetch_profile_avatar', mockResponse);

    const result = await TauriApi.fetchProfileAvatar('npub1example');
    expect(result.data_base64).toBe('AQID');

    const calls = vi.mocked(invoke).mock.calls;
    expect(calls[calls.length - 1][0]).toBe('fetch_profile_avatar');
    expect(calls[calls.length - 1][1]).toEqual({
      request: {
        npub: 'npub1example',
      },
    });
  });
});

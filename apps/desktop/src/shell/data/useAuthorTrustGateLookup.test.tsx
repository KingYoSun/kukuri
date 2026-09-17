import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import type {
  AuthorTrustGateResult,
  CommunityNodeConfig,
  CommunityNodeNodeStatus,
  PostView,
} from '@/lib/api';
import { createShellHookHarness } from '@/shell/testSupport/renderShellHook';

import {
  AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS,
  AUTHOR_TRUST_GATE_LOOKUP_SWEEP_MS,
  useAuthorTrustGateLookup,
} from './useAuthorTrustGateLookup';

// #1061 / ADR 0026 §8.4: 判断は評価の期限まで使い、期限切れ・同意取消では折りたたみを続けない。

const NODE = 'https://node.example';
const AUTHOR = 'b'.repeat(64);

function config(priority: string[] = [NODE]): CommunityNodeConfig {
  return { nodes: [{ base_url: NODE }], trust_node_priority: priority };
}

function status(overrides: Partial<CommunityNodeNodeStatus> = {}): CommunityNodeNodeStatus {
  return {
    base_url: NODE,
    auth_state: { authenticated: true, expires_at: null },
    consent_state: { all_required_accepted: true, items: [] },
    last_error: null,
    ...overrides,
  } as CommunityNodeNodeStatus;
}

function post(authorPubkey = AUTHOR): PostView {
  return { object_id: 'post-1', author_pubkey: authorPubkey } as unknown as PostView;
}

function hiddenGate(expiresAt: string | null): AuthorTrustGateResult {
  return {
    gates: [
      {
        author_pubkey: AUTHOR,
        hidden: true,
        node_base_url: NODE,
        reasons: ['related_users_block_or_mute'],
        expires_at: expiresAt,
        always_visible: false,
      },
    ],
  };
}

type MountProps = {
  config: CommunityNodeConfig;
  posts: PostView[];
  statuses: CommunityNodeNodeStatus[];
};

function mount(result: () => AuthorTrustGateResult, initial?: Partial<MountProps>) {
  const harness = createShellHookHarness();
  const api = { evaluateAuthorTrustGates: vi.fn(async () => result()) };
  const hook = renderHook(
    (props: MountProps) =>
      useAuthorTrustGateLookup({
        api,
        posts: props.posts,
        config: props.config,
        statuses: props.statuses,
      }),
    {
      wrapper: harness.wrapper,
      initialProps: { config: config(), posts: [post()], statuses: [status()], ...initial },
    }
  );
  return { harness, api, hook };
}

async function advance(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

test('no node is queried without an adopted priority', async () => {
  const { harness, api } = mount(() => hiddenGate(null), { config: config([]) });

  await advance(AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS * 2);

  expect(api.evaluateAuthorTrustGates).not.toHaveBeenCalled();
  expect(harness.store.getState().authorTrustGates).toEqual({});
});

test('an expired evaluation is dropped and looked up again', async () => {
  const expired = new Date(Date.now() - 1_000).toISOString();
  const fresh = new Date(Date.now() + 600_000).toISOString();
  let expiresAt = expired;
  const { harness, api } = mount(() => hiddenGate(expiresAt));

  await advance(AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS);
  expect(harness.store.getState().authorTrustGates[AUTHOR]?.hidden).toBe(true);

  // 期限を過ぎた判断はそのまま使わず、照会し直す。
  expiresAt = fresh;
  await advance(AUTHOR_TRUST_GATE_LOOKUP_SWEEP_MS + AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS);
  expect(api.evaluateAuthorTrustGates).toHaveBeenCalledTimes(2);

  // 期限内の判断は作り直さない。
  await advance(AUTHOR_TRUST_GATE_LOOKUP_SWEEP_MS * 2);
  expect(api.evaluateAuthorTrustGates).toHaveBeenCalledTimes(2);
});

test('withdrawing the required consents drops the collapse and stops using it', async () => {
  const fresh = new Date(Date.now() + 600_000).toISOString();
  const { harness, api, hook } = mount(() => hiddenGate(fresh));

  await advance(AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS);
  expect(harness.store.getState().authorTrustGates[AUTHOR]?.hidden).toBe(true);

  api.evaluateAuthorTrustGates.mockImplementation(async () => ({ gates: [] }));
  hook.rerender({
    config: config(),
    posts: [post()],
    statuses: [status({ consent_state: { all_required_accepted: false, items: [] } })],
  });

  // 同意が外れた時点で古い判断を捨てる(照会の応答を待たずに折りたたみをやめる)。
  expect(harness.store.getState().authorTrustGates).toEqual({});
  await advance(AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS);
  expect(harness.store.getState().authorTrustGates).toEqual({});
});

test('a re-render during the debounce does not drop the queued lookup', async () => {
  const fresh = new Date(Date.now() + 600_000).toISOString();
  const { api, hook } = mount(() => hiddenGate(fresh));

  // 待ち時間の途中で表示が更新されると、以前は timer が破棄されたまま再設定されなかった。
  await advance(AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS / 2);
  hook.rerender({ config: config(), posts: [post()], statuses: [status()] });
  await advance(AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS * 2);

  expect(api.evaluateAuthorTrustGates).toHaveBeenCalledTimes(1);
});

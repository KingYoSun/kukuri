import { expect, test, type Page } from '@playwright/test';

const mockIsoTimestamp = '2026-02-25T00:00:00.000Z';

const mockAccount = {
  npub: 'npub1playwrightuser000000000000000000000000000000000000000000000',
  nsec: 'nsec1playwrightuser000000000000000000000000000000000000000000000',
  pubkey: 'f'.repeat(64),
  metadata: {
    npub: 'npub1playwrightuser000000000000000000000000000000000000000000000',
    pubkey: 'f'.repeat(64),
    name: 'Playwright User',
    display_name: 'playwright-user',
    picture: '',
    last_used: mockIsoTimestamp,
    public_profile: true,
    show_online_status: true,
  },
};

const mockTopic = {
  id: 'kukuri:tauri:public',
  name: '#public',
  description: 'Mock public timeline',
  created_at: 1_700_000_000,
  updated_at: 1_700_000_000,
  member_count: 1,
  post_count: 0,
  visibility: 'public',
  is_joined: true,
};

const installTauriMock = async (page: Page) => {
  await page.addInitScript(
    ({ account, topic, timestamp }) => {
      const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));

      const offlineRetryMetrics = {
        totalSuccess: 0,
        totalFailure: 0,
        consecutiveFailure: 0,
        lastOutcome: null,
        lastJobId: null,
        lastJobReason: null,
        lastTrigger: null,
        lastUserPubkey: null,
        lastRetryCount: null,
        lastMaxRetries: null,
        lastBackoffMs: null,
        lastDurationMs: null,
        lastSuccessCount: null,
        lastFailureCount: null,
        lastTimestampMs: null,
      };

      const profileAvatar = {
        npub: account.npub,
        blob_hash: 'mock-blob-hash',
        format: 'png',
        size_bytes: 0,
        access_level: 'public',
        share_ticket: 'mock-share-ticket',
        doc_version: 1,
        updated_at: timestamp,
        content_sha256: 'mock-content-sha256',
        data_base64: '',
      };

      const p2pStatus = {
        connected: true,
        connection_status: 'connected',
        endpoint_id: 'mock-endpoint-id',
        active_topics: [
          {
            topic_id: topic.id,
            peer_count: 0,
            message_count: 0,
            last_activity: topic.updated_at,
          },
        ],
        peer_count: 0,
        peers: [],
        metrics_summary: {
          joins: 0,
          leaves: 0,
          broadcasts_sent: 0,
          messages_received: 0,
        },
      };

      const resolveCommand = (command: string) => {
        switch (command) {
          case 'get_current_account':
            return account;
          case 'list_accounts':
            return [account.metadata];
          case 'initialize_nostr':
          case 'disconnect_nostr':
          case 'initialize_p2p':
          case 'set_bootstrap_nodes':
          case 'clear_bootstrap_nodes':
          case 'connect_to_peer':
          case 'update_cache_metadata':
          case 'clear_community_node_config':
            return null;
          case 'get_relay_status':
          case 'list_nostr_subscriptions':
          case 'get_posts':
          case 'get_topic_timeline':
          case 'get_thread_posts':
          case 'list_pending_topics':
          case 'list_sync_queue_items':
          case 'community_node_list_group_keys':
            return [];
          case 'get_topics':
            return [topic];
          case 'get_topic_stats':
            return {
              topic_id: topic.id,
              member_count: topic.member_count,
              post_count: topic.post_count,
              active_users_24h: 0,
              trending_score: 0,
            };
          case 'get_node_address':
            return {
              addresses: ['/ip4/127.0.0.1/tcp/11223/p2p/mock-node-id'],
            };
          case 'get_p2p_status':
            return p2pStatus;
          case 'get_bootstrap_config':
            return {
              mode: 'default',
              nodes: [],
              effective_nodes: [],
              source: 'fallback',
              env_locked: false,
              cli_nodes: [],
              cli_updated_at_ms: null,
            };
          case 'list_direct_message_conversations':
            return {
              items: [],
              next_cursor: null,
              has_more: false,
            };
          case 'fetch_profile_avatar':
            return profileAvatar;
          case 'profile_avatar_sync':
            return {
              npub: account.npub,
              current_version: 1,
              updated: false,
              avatar: null,
            };
          case 'get_cache_status':
            return {
              total_items: 0,
              stale_items: 0,
              cache_types: [],
            };
          case 'get_offline_retry_metrics':
          case 'record_offline_retry_outcome':
            return offlineRetryMetrics;
          case 'add_to_sync_queue':
            return 1;
          case 'get_offline_actions':
            return [];
          case 'sync_offline_actions':
            return {
              syncedCount: 0,
              failedCount: 0,
              pendingCount: 0,
            };
          case 'cleanup_expired_cache':
            return 0;
          case 'save_optimistic_update':
            return 'mock-optimistic-update-id';
          case 'rollback_optimistic_update':
            return null;
          case 'get_community_node_config':
            return { nodes: [] };
          case 'community_node_get_trust_provider':
            return null;
          case 'community_node_get_consent_status':
            return { accepted_policies: [], required_policies: [] };
          case 'access_control_list_join_requests':
            return { items: [] };
          default:
            return null;
        }
      };

      Object.defineProperty(window, '__TAURI_INTERNALS__', {
        configurable: true,
        writable: true,
        value: {
          invoke: async (command: string) => clone(resolveCommand(command)),
          convertFileSrc: (filePath: string) => filePath,
          unregisterCallback: () => {},
        },
      });
    },
    {
      account: mockAccount,
      topic: mockTopic,
      timestamp: mockIsoTimestamp,
    },
  );
};

const openHomePage = async (page: Page) => {
  await page.goto('/');
  await expect(page.getByTestId('home-page')).toBeVisible({ timeout: 30_000 });
};

test.describe('Critical path smoke', () => {
  test.beforeEach(async ({ page }) => {
    await installTauriMock(page);
  });

  test('connection status is visible', async ({ page }) => {
    await openHomePage(page);
    await expect(page.getByTestId('sync-indicator')).toBeVisible();
  });

  test('settings page loads from sidebar', async ({ page }) => {
    await openHomePage(page);

    await page.getByTestId('open-settings-button').click();
    await expect(page).toHaveURL(/\/settings/);
    await expect(page.getByTestId('settings-page')).toBeVisible();
  });

  test('can navigate between core screens', async ({ page }) => {
    await openHomePage(page);

    await page.getByTestId('category-topics').click();
    await expect(page).toHaveURL(/\/topics/);

    await page.getByTestId('category-search').click();
    await expect(page).toHaveURL(/\/search/);
  });
});

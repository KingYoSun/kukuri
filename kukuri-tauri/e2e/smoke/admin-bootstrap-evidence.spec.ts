import { expect, test } from '@playwright/test';

type EvidencePhase = 'before' | 'after';

const adminBaseUrl = process.env.PLAYWRIGHT_ADMIN_BASE_URL ?? 'http://127.0.0.1:4173';

const adminUser = {
  admin_user_id: 'admin-e2e',
  username: 'admin-e2e',
};

const servicesResponse = [
  {
    service: 'relay',
    version: '0.1.0',
    config_json: {},
    updated_at: 1_700_000_000,
    updated_by: 'e2e',
    health: {
      status: 'healthy',
      checked_at: 1_700_000_010,
      details: {},
    },
  },
];

const dashboardResponse = {
  collected_at: 1_700_000_020,
  outbox_backlog: {
    max_seq: 0,
    total_backlog: 0,
    max_backlog: 0,
    threshold: 100,
    alert: false,
    consumers: [],
  },
  reject_surge: {
    source_status: 'healthy',
    source_error: null,
    current_total: 0,
    previous_total: 0,
    delta: 0,
    per_minute: 0,
    threshold_per_minute: 50,
    alert: false,
  },
  db_pressure: {
    db_size_bytes: 1_024 * 1_024,
    disk_soft_limit_bytes: 2_048 * 1_024,
    disk_utilization: 0.5,
    active_connections: 1,
    max_connections: 20,
    connection_utilization: 0.05,
    lock_waiters: 0,
    connection_threshold: 0.8,
    lock_waiter_threshold: 5,
    alert: false,
    alerts: [],
  },
};

const nodeSubscriptionsByPhase: Record<EvidencePhase, unknown[]> = {
  before: [
    {
      topic_id: 'kukuri:evidence',
      enabled: true,
      ref_count: 1,
      ingest_policy: null,
      connected_nodes: ['n0@bootstrap.kukuri.dev:11223'],
      connected_node_count: 1,
      connected_users: [],
      connected_user_count: 0,
      updated_at: 1_700_000_100,
    },
  ],
  after: [
    {
      topic_id: 'kukuri:evidence',
      enabled: true,
      ref_count: 1,
      ingest_policy: null,
      connected_nodes: ['node-added@127.0.0.1:11223'],
      connected_node_count: 1,
      connected_users: [
        'npub1bootstrapconnecteduser0000000000000000000000000000000000000000000',
      ],
      connected_user_count: 1,
      updated_at: 1_700_000_200,
    },
  ],
};

const subscriptionsByPhase: Record<EvidencePhase, unknown[]> = {
  before: [],
  after: [
    {
      subscription_id: 'sub-active',
      subscriber_pubkey: 'npub1bootstrapconnecteduser0000000000000000000000000000000000000000000',
      plan_id: 'basic',
      status: 'active',
      started_at: 1_700_000_200,
      ended_at: null,
    },
  ],
};

const jsonResponse = (payload: unknown) => ({
  status: 200,
  contentType: 'application/json',
  body: JSON.stringify(payload),
});

test.describe('Admin bootstrap evidence smoke', () => {
  test('captures bootstrap switch before/after with connected user count', async ({ page }) => {
    let phase: EvidencePhase = 'before';

    await page.route('**/admin-api/v1/admin/auth/me', async (route) => {
      await route.fulfill(jsonResponse(adminUser));
    });
    await page.route('**/admin-api/v1/admin/services', async (route) => {
      await route.fulfill(jsonResponse(servicesResponse));
    });
    await page.route('**/admin-api/v1/admin/dashboard', async (route) => {
      await route.fulfill(jsonResponse(dashboardResponse));
    });
    await page.route(/\/admin-api\/v1\/admin\/node-subscriptions(?:\?.*)?$/, async (route) => {
      await route.fulfill(jsonResponse(nodeSubscriptionsByPhase[phase]));
    });
    await page.route(/\/admin-api\/v1\/admin\/subscriptions(?:\?.*)?$/, async (route) => {
      await route.fulfill(jsonResponse(subscriptionsByPhase[phase]));
    });

    await page.goto(adminBaseUrl);

    await expect(page.getByRole('heading', { name: 'Bootstrap' })).toBeVisible();
    await expect(page.getByText('Connected users: 0')).toBeVisible();
    await expect(page.getByText('n0@bootstrap.kukuri.dev:11223')).toBeVisible();

    await page.screenshot({
      path: test.info().outputPath('admin-bootstrap-before-switch.png'),
      fullPage: true,
    });

    phase = 'after';
    await page.reload();

    await expect(page.getByText('Connected users: 1')).toBeVisible();
    await expect(page.getByText('node-added@127.0.0.1:11223')).toBeVisible();
    await expect(
      page.getByText('npub1bootstrapconnecteduser0000000000000000000000000000000000000000000'),
    ).toBeVisible();
    await expect(page.getByText('n0@bootstrap.kukuri.dev:11223')).toHaveCount(0);

    await page.screenshot({
      path: test.info().outputPath('admin-bootstrap-after-switch.png'),
      fullPage: true,
    });
  });
});

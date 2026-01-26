import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { randomUUID } from 'node:crypto';
import type { AddressInfo } from 'node:net';

type MockState = {
  accessToken: string | null;
  pubkey: string | null;
  challenge: string | null;
};

let server: ReturnType<typeof createServer> | null = null;
let baseUrl = '';
const state: MockState = {
  accessToken: null,
  pubkey: null,
  challenge: null,
};

const nowSeconds = () => Math.floor(Date.now() / 1000);

const nextId = () =>
  typeof randomUUID === 'function'
    ? randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`;

const sendJson = (res: ServerResponse, status: number, payload: unknown) => {
  res.statusCode = status;
  res.setHeader('content-type', 'application/json');
  res.end(JSON.stringify(payload));
};

const readJson = async (req: IncomingMessage): Promise<unknown> => {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(chunk as Buffer);
  }
  if (chunks.length === 0) {
    return null;
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString('utf-8'));
  } catch {
    return null;
  }
};

const requireAuth = (req: IncomingMessage, res: ServerResponse): boolean => {
  const header = req.headers.authorization ?? '';
  const token = header.startsWith('Bearer ') ? header.slice('Bearer '.length) : '';
  if (!state.accessToken || token !== state.accessToken) {
    sendJson(res, 401, { error: 'unauthorized' });
    return false;
  }
  return true;
};

const handleRequest = async (req: IncomingMessage, res: ServerResponse): Promise<void> => {
  const method = req.method ?? 'GET';
  const url = new URL(req.url ?? '/', 'http://127.0.0.1');
  const path = url.pathname;

  if (method === 'POST' && path === '/v1/auth/challenge') {
    const payload = (await readJson(req)) as { pubkey?: string } | null;
    state.challenge = `challenge-${nextId()}`;
    if (payload?.pubkey) {
      state.pubkey = payload.pubkey;
    }
    sendJson(res, 200, { challenge: state.challenge, expires_at: nowSeconds() + 600 });
    return;
  }

  if (method === 'POST' && path === '/v1/auth/verify') {
    const payload = (await readJson(req)) as { auth_event_json?: { pubkey?: string } } | null;
    const pubkey = payload?.auth_event_json?.pubkey ?? state.pubkey ?? `npub-e2e-${nextId()}`;
    state.pubkey = pubkey;
    state.accessToken = `token-${nextId()}`;
    sendJson(res, 200, {
      access_token: state.accessToken,
      token_type: 'bearer',
      expires_at: nowSeconds() + 3600,
      pubkey,
    });
    return;
  }

  if (method === 'GET' && path === '/v1/consents/status') {
    if (!requireAuth(req, res)) {
      return;
    }
    sendJson(res, 200, {
      policies: [{ id: 'default', version: 1, required: true }],
      accepted: true,
      accepted_at: nowSeconds(),
    });
    return;
  }

  if (method === 'POST' && path === '/v1/consents') {
    if (!requireAuth(req, res)) {
      return;
    }
    sendJson(res, 200, { accepted: true, accepted_at: nowSeconds() });
    return;
  }

  if (method === 'GET' && path === '/v1/labels') {
    if (!requireAuth(req, res)) {
      return;
    }
    sendJson(res, 200, { items: [] });
    return;
  }

  if (method === 'GET' && path === '/v1/trust/report-based') {
    if (!requireAuth(req, res)) {
      return;
    }
    sendJson(res, 200, { score: 0.82 });
    return;
  }

  if (method === 'GET' && path === '/v1/trust/communication-density') {
    if (!requireAuth(req, res)) {
      return;
    }
    sendJson(res, 200, { score: 0.64 });
    return;
  }

  if (method === 'GET' && path === '/v1/search') {
    if (!requireAuth(req, res)) {
      return;
    }
    sendJson(res, 200, { items: [], cursor: null });
    return;
  }

  if (method === 'GET' && path === '/v1/keys/envelopes') {
    if (!requireAuth(req, res)) {
      return;
    }
    sendJson(res, 200, { items: [] });
    return;
  }

  if (method === 'POST' && path === '/v1/reports') {
    if (!requireAuth(req, res)) {
      return;
    }
    sendJson(res, 200, { accepted: true });
    return;
  }

  if (method === 'GET' && path === '/v1/bootstrap/nodes') {
    sendJson(res, 200, {
      nodes: [
        {
          node: 'bootstrap@127.0.0.1:11223',
          role: 'bootstrap',
        },
      ],
    });
    return;
  }

  if (method === 'GET' && path.startsWith('/v1/bootstrap/topics/') && path.endsWith('/services')) {
    const topicId = path.split('/').slice(4, -1).join('/') || 'unknown';
    sendJson(res, 200, { topic_id: topicId, services: [] });
    return;
  }

  sendJson(res, 404, { error: 'not_found' });
};

export const startCommunityNodeMock = async (): Promise<{ baseUrl: string }> => {
  if (server) {
    return { baseUrl };
  }

  server = createServer((req, res) => {
    void handleRequest(req, res).catch(() => {
      sendJson(res, 500, { error: 'internal_error' });
    });
  });

  await new Promise<void>((resolve, reject) => {
    server?.once('error', reject);
    server?.listen(0, '127.0.0.1', () => resolve());
  });

  const address = server.address() as AddressInfo | null;
  if (!address || typeof address.port !== 'number') {
    throw new Error('Community node mock server failed to bind');
  }
  baseUrl = `http://127.0.0.1:${address.port}`;
  return { baseUrl };
};

export const stopCommunityNodeMock = async (): Promise<void> => {
  if (!server) {
    return;
  }
  await new Promise<void>((resolve, reject) => {
    server?.close((err) => {
      if (err) {
        reject(err);
        return;
      }
      resolve();
    });
  });
  server = null;
  baseUrl = '';
  state.accessToken = null;
  state.pubkey = null;
  state.challenge = null;
};

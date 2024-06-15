/**
 * Welcome to Cloudflare Workers! This is your first worker.
 *
 * - Run `npm run dev` in your terminal to start a development server
 * - Open a browser tab at http://localhost:8787/ to see your worker in action
 * - Run `npm run deploy` to publish your worker
 *
 * Bind resources to your worker in `wrangler.toml`. After adding bindings, a type definition for the
 * `Env` object can be regenerated with `npm run cf-typegen`.
 *
 * Learn more at https://developers.cloudflare.com/workers/
 */

import { Hono } from 'hono';
import { drizzle } from 'drizzle-orm/d1';
import { peers } from './schema';
import { eq, and, lt } from 'drizzle-orm';
import { PeerPoolIndexReq, PeerPoolCreateReq } from 'common/types/PeerPool';
import { D1Database, type ExportedHandlerScheduledHandler } from '@cloudflare/workers-types';
import dayjs from 'dayjs';

type Bindings = {
	DB: D1Database;
};

type Env = {
	DB: D1Database;
};

const app = new Hono<{ Bindings: Bindings }>();

app.get('/', (c) => c.text('Hello World!'));

app.get('/peers', async (c) => {
	const params = await c.req.json<PeerPoolIndexReq>();
	const db = drizzle(c.env.DB);
	const result = await db.select().from(peers).where(eq(peers.topic, params.topic));
	return c.json(result);
});

app.post('/peers', async (c) => {
	const params = await c.req.json<PeerPoolCreateReq>();
	const db = drizzle(c.env.DB);
	const result = await db.insert(peers).values({ topic: params.topic, maddr: params.maddr });
	return c.json(result);
});

app.put('/peers/:id', async (c) => {
	const id = parseInt(c.req.param('id'));

	if (isNaN(id)) return c.json({ error: 'Invalid ID' }, 400);

	const params = await c.req.json<PeerPoolCreateReq>();
	const db = drizzle(c.env.DB);
	const result = db.update(peers).set({ topic: params.topic, maddr: params.maddr, connectionCount: params.connectionCount });
	return c.json(result);
});

const scheduled: ExportedHandlerScheduledHandler<Env> = async (event, env) => {
	const db = drizzle(env.DB);

	const result = await db
		.delete(peers)
		.where(and(eq(peers.connectionCount, 0), lt(peers.updatedAt, dayjs().subtract(5, 'minutes').toDate())));
	console.log(`cron delete finished: ${result.success ? JSON.stringify(result.results) : JSON.stringify(result.error)}`);
};

export default {
	fetch: app.fetch,
	scheduled,
};

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

import { Hono, type Context, type Next } from 'hono';
import { cors } from 'hono/cors';
import { HTTPException } from 'hono/http-exception';
import { drizzle } from 'drizzle-orm/d1';
import { peers } from './schema';
import { eq, and, lt, count } from 'drizzle-orm';
import { type D1Database, type ExportedHandlerScheduledHandler } from '@cloudflare/workers-types';
import dayjs from 'dayjs';

type Bindings = {
	DB: D1Database;
};

type Env = {
	DB: D1Database;
};

const app = new Hono<{ Bindings: Bindings }>();

app.use('/peers/*', async (c: Context<{ Bindings: Bindings }>, next: Next) =>
	cors({
		origin: '*',
		allowHeaders: ['Content-Type', 'Authorization'],
		allowMethods: ['GET', 'POST', 'OPTIONS'],
		maxAge: 600,
		credentials: true,
	})(c, next),
);

app.get('/', (c) => c.text('Hello World!'));

app.get('/peers', async (c) => {
	const topic = await c.req.query('topic');
	if (!topic) {
		throw new HTTPException(500, { message: 'topic is not valid' });
	}

	const db = drizzle(c.env.DB);
	const result = await db.select().from(peers).where(eq(peers.topic, topic));
	return c.json(result);
});

app.get('/peers/count', async (c) => {
	const topic = await c.req.query('topic');
	if (!topic) {
		throw new HTTPException(500, { message: 'topic is not valid' });
	}

	const db = drizzle(c.env.DB);
	const result = await db.select({ count: count() }).from(peers).where(eq(peers.topic, topic));
	return c.json(result[0]);
});

app.post('/peers', async (c) => {
	const params = await c.req.json<typeof peers.$inferInsert>();
	const db = drizzle(c.env.DB);
	const result = await db.insert(peers).values({ topic: params.topic, maddr: params.maddr }).returning();
	return c.json(result);
});

app.put('/peers/:id', async (c) => {
	const id = parseInt(c.req.param('id'));

	if (isNaN(id)) return c.json({ error: 'Invalid ID' }, 400);

	const params = await c.req.json<typeof peers.$inferInsert>();
	const db = drizzle(c.env.DB);
	db.update(peers)
		.set({ topic: params.topic, maddr: params.maddr, connectionCount: params.connectionCount })
		.where(eq(peers.id, id))
		.returning();
	return c.json([{ id: id }]);
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

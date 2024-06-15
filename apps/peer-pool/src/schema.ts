import { sql } from 'drizzle-orm';
import { sqliteTable, integer, text } from 'drizzle-orm/sqlite-core';

export const peers = sqliteTable('peers', {
	id: integer('id', { mode: 'number' }).primaryKey({ autoIncrement: true }),
	topic: text('topic').notNull(),
	maddr: text('maddr').notNull(),
	connectionCount: integer('connection_count').default(0),
	createdAt: integer('created_at', { mode: 'timestamp' }).default(sql`(strftime('%s', 'now'))`),
	updatedAt: integer('updated_at', { mode: 'timestamp' }).default(sql`(strftime('%s', 'now'))`),
});

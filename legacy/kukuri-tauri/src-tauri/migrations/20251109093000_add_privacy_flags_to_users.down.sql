BEGIN TRANSACTION;

CREATE TABLE users_backup AS
SELECT
    npub,
    pubkey,
    display_name,
    bio,
    avatar_url,
    created_at,
    updated_at
FROM users;

DROP TABLE users;

CREATE TABLE users (
    npub TEXT PRIMARY KEY NOT NULL,
    pubkey TEXT NOT NULL UNIQUE,
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

INSERT INTO users (
    npub,
    pubkey,
    display_name,
    bio,
    avatar_url,
    created_at,
    updated_at
)
SELECT
    npub,
    pubkey,
    display_name,
    bio,
    avatar_url,
    created_at,
    updated_at
FROM users_backup;

DROP TABLE users_backup;

COMMIT;

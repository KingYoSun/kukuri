ALTER TABLE bookmarked_custom_reactions
    ADD COLUMN search_key TEXT NOT NULL DEFAULT '';

UPDATE bookmarked_custom_reactions
SET search_key = asset_id
WHERE trim(search_key) = '';

CREATE INDEX IF NOT EXISTS idx_reaction_cache_author_updated_at
    ON reaction_cache(author_pubkey, updated_at DESC, reaction_id DESC);

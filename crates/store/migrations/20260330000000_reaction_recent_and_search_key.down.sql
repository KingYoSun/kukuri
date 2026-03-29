DROP INDEX IF EXISTS idx_reaction_cache_author_updated_at;
ALTER TABLE bookmarked_custom_reactions
    DROP COLUMN search_key;

ALTER TABLE cn_search.post_search_documents
    DROP CONSTRAINT IF EXISTS post_search_documents_pkey;

ALTER TABLE cn_search.post_search_documents
    ADD CONSTRAINT post_search_documents_pkey PRIMARY KEY (post_id, topic_id);

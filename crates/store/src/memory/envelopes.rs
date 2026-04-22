use super::*;

impl MemoryStore {
    pub(super) async fn store_put_envelope_impl(&self, envelope: KukuriEnvelope) -> Result<()> {
        let topic_id = envelope.topic_id().map(|topic| topic.0);
        let thread_ref = envelope.thread_ref();
        self.envelopes
            .write()
            .await
            .insert(envelope.id.clone(), envelope.clone());

        if let Some(topic_id) = topic_id {
            self.topic_objects
                .write()
                .await
                .entry(topic_id.clone())
                .or_default()
                .push(envelope.id.clone());

            let root = thread_ref
                .as_ref()
                .map(|thread| thread.root.clone())
                .unwrap_or_else(|| envelope.id.clone());
            self.object_threads
                .write()
                .await
                .entry(topic_id)
                .or_default()
                .insert(envelope.id.0.clone(), root);
        }

        if let Some(profile) = parse_profile(&envelope)? {
            self.upsert_profile(profile).await?;
        }
        if let Some(edge) = parse_follow_edge(&envelope)? {
            self.upsert_follow_edge(edge).await?;
        }

        Ok(())
    }

    pub(super) async fn store_get_envelope_impl(
        &self,
        envelope_id: &EnvelopeId,
    ) -> Result<Option<KukuriEnvelope>> {
        Ok(self.envelopes.read().await.get(envelope_id).cloned())
    }

    pub(super) async fn store_list_topic_timeline_impl(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>> {
        let envelopes = self.envelopes.read().await;
        let mut items = self
            .topic_objects
            .read()
            .await
            .get(topic_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|object_id| envelopes.get(&object_id).cloned())
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        let filtered = apply_desc_cursor(items, cursor, limit);
        Ok(filtered)
    }

    pub(super) async fn store_list_thread_impl(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>> {
        let envelopes = self.envelopes.read().await;
        let roots = self.object_threads.read().await;
        let mut items = roots
            .get(topic_id)
            .into_iter()
            .flat_map(|entries| entries.keys())
            .filter_map(|object_id| {
                envelopes
                    .get(&EnvelopeId::from(object_id.as_str()))
                    .cloned()
            })
            .filter(|envelope| {
                envelope.id == *thread_root_object_id
                    || envelope
                        .thread_ref()
                        .map(|thread| thread.root == *thread_root_object_id)
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            let left_root = left.id == *thread_root_object_id;
            let right_root = right.id == *thread_root_object_id;
            left_root
                .cmp(&right_root)
                .reverse()
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.cmp(&right.id))
        });
        let filtered = apply_asc_cursor(items, cursor, limit);
        Ok(filtered)
    }
}

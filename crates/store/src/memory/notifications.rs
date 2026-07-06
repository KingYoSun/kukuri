use super::*;

#[async_trait]
impl NotificationStore for MemoryStore {
    async fn put_notification_if_absent(&self, row: NotificationRow) -> Result<bool> {
        let mut notifications = self.notification_rows.write().await;
        if notifications.contains_key(row.notification_id.as_str()) {
            return Ok(false);
        }
        notifications.insert(row.notification_id.clone(), row);
        Ok(true)
    }

    async fn list_notifications(&self) -> Result<Vec<NotificationRow>> {
        let mut items = self
            .notification_rows
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .received_at
                .cmp(&left.received_at)
                .then_with(|| right.notification_id.cmp(&left.notification_id))
        });
        Ok(items)
    }

    async fn mark_notification_read(&self, notification_id: &str, read_at: i64) -> Result<()> {
        if let Some(row) = self
            .notification_rows
            .write()
            .await
            .get_mut(notification_id)
        {
            row.read_at.get_or_insert(read_at);
        }
        Ok(())
    }

    async fn mark_all_notifications_read(&self, read_at: i64) -> Result<()> {
        for row in self.notification_rows.write().await.values_mut() {
            row.read_at.get_or_insert(read_at);
        }
        Ok(())
    }

    async fn count_unread_notifications(&self) -> Result<usize> {
        Ok(self
            .notification_rows
            .read()
            .await
            .values()
            .filter(|row| row.read_at.is_none())
            .count())
    }
}

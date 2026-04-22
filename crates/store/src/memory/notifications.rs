use super::*;

impl MemoryStore {
    pub(super) async fn projection_put_notification_if_absent_impl(
        &self,
        row: NotificationRow,
    ) -> Result<bool> {
        let mut notifications = self.notification_rows.write().await;
        if notifications.contains_key(row.notification_id.as_str()) {
            return Ok(false);
        }
        notifications.insert(row.notification_id.clone(), row);
        Ok(true)
    }

    pub(super) async fn projection_list_notifications_impl(&self) -> Result<Vec<NotificationRow>> {
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

    pub(super) async fn projection_mark_notification_read_impl(
        &self,
        notification_id: &str,
        read_at: i64,
    ) -> Result<()> {
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

    pub(super) async fn projection_mark_all_notifications_read_impl(
        &self,
        read_at: i64,
    ) -> Result<()> {
        for row in self.notification_rows.write().await.values_mut() {
            row.read_at.get_or_insert(read_at);
        }
        Ok(())
    }

    pub(super) async fn projection_count_unread_notifications_impl(&self) -> Result<usize> {
        Ok(self
            .notification_rows
            .read()
            .await
            .values()
            .filter(|row| row.read_at.is_none())
            .count())
    }
}

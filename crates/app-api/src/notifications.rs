use crate::service::*;

impl AppService {
    pub async fn list_notifications(&self) -> Result<Vec<NotificationView>> {
        let mut items = Vec::new();
        for row in self.projection_store.list_notifications().await? {
            items.push(self.notification_view_from_row(row).await?);
        }
        Ok(items)
    }

    pub async fn mark_notification_read(
        &self,
        notification_id: &str,
    ) -> Result<NotificationStatusView> {
        self.projection_store
            .mark_notification_read(notification_id, Utc::now().timestamp_millis())
            .await?;
        self.notification_status_view().await
    }

    pub async fn mark_all_notifications_read(&self) -> Result<NotificationStatusView> {
        self.projection_store
            .mark_all_notifications_read(Utc::now().timestamp_millis())
            .await?;
        self.notification_status_view().await
    }

    pub async fn get_notification_status(&self) -> Result<NotificationStatusView> {
        self.notification_status_view().await
    }
}

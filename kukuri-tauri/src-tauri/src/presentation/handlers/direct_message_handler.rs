use crate::application::services::{
    DirectMessagePageResult, DirectMessageService, DirectMessageServiceDirection,
    SendDirectMessageResult,
};
use crate::domain::entities::DirectMessage;
use crate::presentation::dto::direct_message_dto::{
    DirectMessageConversationListDto, DirectMessageConversationSummaryDto, DirectMessageDto,
    DirectMessagePage, ListDirectMessageConversationsRequest, ListDirectMessagesRequest,
    MarkDirectMessageConversationReadRequest, MessagePageDirection as RequestDirection,
    SendDirectMessageRequest, SendDirectMessageResponse,
};
use crate::shared::AppError;
use std::sync::Arc;

pub struct DirectMessageHandler {
    service: Arc<DirectMessageService>,
}

impl DirectMessageHandler {
    pub fn new(service: Arc<DirectMessageService>) -> Self {
        Self { service }
    }

    pub async fn send_direct_message(
        &self,
        owner_npub: &str,
        request: SendDirectMessageRequest,
    ) -> Result<SendDirectMessageResponse, AppError> {
        let result = self
            .service
            .send_direct_message(
                owner_npub,
                &request.recipient_npub,
                &request.content,
                request.client_message_id.clone(),
            )
            .await?;

        Ok(to_send_response(result))
    }

    pub async fn list_direct_messages(
        &self,
        owner_npub: &str,
        request: ListDirectMessagesRequest,
    ) -> Result<DirectMessagePage, AppError> {
        let limit = request.limit.map(|value| value as usize);
        let direction = request
            .direction
            .map(map_direction)
            .unwrap_or(DirectMessageServiceDirection::Backward);

        let page = self
            .service
            .list_direct_messages(
                owner_npub,
                &request.conversation_npub,
                request.cursor.as_deref(),
                limit,
                direction,
            )
            .await?;

        Ok(to_page_dto(page))
    }

    pub async fn list_direct_message_conversations(
        &self,
        owner_npub: &str,
        request: ListDirectMessageConversationsRequest,
    ) -> Result<DirectMessageConversationListDto, AppError> {
        let limit = request.limit.map(|value| value as usize);
        let summaries = self
            .service
            .list_direct_message_conversations(owner_npub, limit)
            .await?;

        let items = summaries
            .into_iter()
            .map(|summary| DirectMessageConversationSummaryDto {
                conversation_npub: summary.conversation_npub,
                unread_count: summary.unread_count,
                last_read_at: summary.last_read_at,
                last_message: summary.last_message.map(map_direct_message_to_dto),
            })
            .collect();

        Ok(DirectMessageConversationListDto { items })
    }

    pub async fn mark_conversation_as_read(
        &self,
        owner_npub: &str,
        request: MarkDirectMessageConversationReadRequest,
    ) -> Result<(), AppError> {
        self.service
            .mark_conversation_as_read(owner_npub, &request.conversation_npub, request.last_read_at)
            .await
    }
}

fn to_send_response(result: SendDirectMessageResult) -> SendDirectMessageResponse {
    SendDirectMessageResponse {
        event_id: result.event_id,
        queued: result.queued,
    }
}

fn to_page_dto(page: DirectMessagePageResult) -> DirectMessagePage {
    let items = page
        .items
        .into_iter()
        .map(map_direct_message_to_dto)
        .collect();

    DirectMessagePage {
        items,
        next_cursor: page.next_cursor,
        has_more: page.has_more,
    }
}

fn map_direction(direction: RequestDirection) -> DirectMessageServiceDirection {
    match direction {
        RequestDirection::Backward => DirectMessageServiceDirection::Backward,
        RequestDirection::Forward => DirectMessageServiceDirection::Forward,
    }
}

fn map_direct_message_to_dto(message: DirectMessage) -> DirectMessageDto {
    let content = message.decrypted_content.clone().unwrap_or_default();
    DirectMessageDto {
        event_id: message.event_id.clone(),
        client_message_id: message.client_message_id.clone(),
        sender_npub: message.sender_npub.clone(),
        recipient_npub: message.recipient_npub.clone(),
        content,
        created_at: message.created_at_millis(),
        delivered: message.delivered,
    }
}

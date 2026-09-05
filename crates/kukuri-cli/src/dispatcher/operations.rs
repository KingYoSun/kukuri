use std::{future::Future, sync::Arc};

use kukuri_desktop_runtime::ClientHost;
use tokio::{
    sync::{Mutex, Semaphore, oneshot},
    task::JoinSet,
};

use super::{DispatchReply, Dispatcher};
use crate::{
    protocol::{
        CommandEffect, ProtocolError, RequestEnvelope, ResponseEnvelope, SecretInput, error_code,
    },
    registry::CommandOutput,
};

/// 接続のtimeoutや切断で、変更操作のrollback・cleanupを破棄しない。
pub(super) struct OperationTasks {
    tasks: Mutex<JoinSet<()>>,
    permits: Arc<Semaphore>,
}

impl OperationTasks {
    pub(super) fn new() -> Self {
        Self {
            tasks: Mutex::new(JoinSet::new()),
            permits: Arc::new(Semaphore::new(64)),
        }
    }

    async fn run<F>(&self, operation: F) -> Result<CommandOutput, ProtocolError>
    where
        F: Future<Output = Result<CommandOutput, ProtocolError>> + Send + 'static,
    {
        let permit = self.permits.clone().try_acquire_owned().map_err(|_| {
            ProtocolError::new(
                error_code::BACKPRESSURE,
                "mutation limit reached or daemon is stopping",
            )
        })?;
        let (send, receive) = oneshot::channel();
        {
            let mut tasks = self.tasks.lock().await;
            if self.permits.is_closed() {
                return Err(ProtocolError::new(
                    error_code::ACTION_REQUIRED,
                    "daemon is stopping",
                ));
            }
            while tasks.try_join_next().is_some() {}
            tasks.spawn(async move {
                let _permit = permit;
                let result = operation.await;
                let _ = send.send(result);
            });
        }
        receive.await.map_err(|_| {
            ProtocolError::new(
                error_code::OPERATION_OUTCOME_UNKNOWN,
                "mutation task did not return a result",
            )
        })?
    }

    async fn finish(&self) {
        self.permits.close();
        let mut tasks = self.tasks.lock().await;
        while tasks.join_next().await.is_some() {}
    }
}

impl Dispatcher {
    pub async fn dispatch(
        &self,
        request: RequestEnvelope,
        secret: Option<SecretInput>,
        expected_profile: &str,
        host: Option<&Arc<ClientHost>>,
    ) -> DispatchReply {
        let mutating = request.command != "cancel_device_backup"
            && self
                .registry
                .get(&request.command)
                .is_some_and(|registration| {
                    matches!(
                        registration.metadata.effect,
                        CommandEffect::Write | CommandEffect::Destructive
                    )
                });
        let result = if mutating {
            let dispatcher = self.clone();
            let owned_request = request.clone();
            let profile = expected_profile.to_owned();
            let host = host.cloned();
            self.operations
                .run(async move {
                    dispatcher
                        .dispatch_inner(&owned_request, secret, &profile, host.as_ref())
                        .await
                })
                .await
        } else {
            self.dispatch_inner(&request, secret, expected_profile, host)
                .await
        };
        match result {
            Ok(CommandOutput::Unary(data)) => {
                DispatchReply::Unary(ResponseEnvelope::success(&request, data), None)
            }
            Ok(CommandOutput::Secret { data, secret }) => {
                let mut response = ResponseEnvelope::success(&request, data);
                response.secret_bytes = Some(secret.expose().len() as u64);
                DispatchReply::Unary(response, Some(secret))
            }
            Ok(CommandOutput::Events(receiver)) => DispatchReply::Events { request, receiver },
            Err(error) => DispatchReply::Unary(ResponseEnvelope::failure(&request, error), None),
        }
    }

    /// 新規変更操作を閉じ、進行中の操作が安全な終端へ達してからhostを停止する。
    pub async fn finish_operations(&self) {
        self.operations.finish().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Notify;

    #[tokio::test]
    async fn mutation_tasks_keep_capacity_until_completion_and_reject_after_shutdown() {
        let operations = OperationTasks::new();
        assert_eq!(operations.permits.available_permits(), 64);
        let held = operations
            .permits
            .clone()
            .acquire_many_owned(64)
            .await
            .expect("capacity");
        let error = operations
            .run(async { Ok(CommandOutput::Unary(serde_json::Value::Null)) })
            .await
            .err()
            .expect("capacity拒否");
        assert_eq!(error.code, error_code::BACKPRESSURE);
        drop(held);
        operations.finish().await;
        assert!(
            operations
                .run(async { panic!("停止後は実行しない") })
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn shutdown_waits_for_detached_mutation_cleanup() {
        let operations = Arc::new(OperationTasks::new());
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let caller = tokio::spawn({
            let operations = operations.clone();
            let entered = entered.clone();
            let release = release.clone();
            async move {
                operations
                    .run(async move {
                        entered.notify_one();
                        release.notified().await;
                        Ok(CommandOutput::Unary(serde_json::Value::Null))
                    })
                    .await
            }
        });
        entered.notified().await;
        caller.abort();
        let _ = caller.await;
        let mut shutdown = tokio::spawn({
            let operations = operations.clone();
            async move { operations.finish().await }
        });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut shutdown)
                .await
                .is_err()
        );
        release.notify_one();
        tokio::time::timeout(std::time::Duration::from_secs(5), shutdown)
            .await
            .expect("drain")
            .expect("shutdown");
        assert_eq!(operations.permits.available_permits(), 64);
    }
}

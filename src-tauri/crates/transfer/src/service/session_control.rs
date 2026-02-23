use super::pipeline::{TerminalPersistOptions, TransferHistorySyncReason};
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionControlOperation {
    Pause,
    Resume,
    Cancel,
}

impl SessionControlOperation {
    fn not_running_error(self) -> AppError {
        match self {
            Self::Pause => AppError::new("transfer_session_not_running", "会话未运行，无法暂停"),
            Self::Resume => AppError::new("transfer_session_not_running", "会话未运行，无法继续"),
            Self::Cancel => AppError::new("transfer_session_not_running", "会话未运行，无法取消"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SessionControlTransition {
    status: TransferStatus,
    finished_at: Option<i64>,
}

fn resolve_session_control_transition(
    current_status: TransferStatus,
    operation: SessionControlOperation,
    timestamp: i64,
) -> AppResult<SessionControlTransition> {
    match operation {
        SessionControlOperation::Pause => match current_status {
            TransferStatus::Queued | TransferStatus::Running | TransferStatus::Paused => {
                Ok(SessionControlTransition {
                    status: TransferStatus::Paused,
                    finished_at: None,
                })
            }
            _ => Err(operation.not_running_error()),
        },
        SessionControlOperation::Resume => match current_status {
            TransferStatus::Queued | TransferStatus::Running | TransferStatus::Paused => {
                Ok(SessionControlTransition {
                    status: TransferStatus::Running,
                    finished_at: None,
                })
            }
            _ => Err(operation.not_running_error()),
        },
        SessionControlOperation::Cancel => match current_status {
            TransferStatus::Queued | TransferStatus::Running | TransferStatus::Paused => {
                Ok(SessionControlTransition {
                    status: TransferStatus::Canceled,
                    finished_at: Some(timestamp),
                })
            }
            TransferStatus::Canceled => Ok(SessionControlTransition {
                status: TransferStatus::Canceled,
                finished_at: None,
            }),
            _ => Err(operation.not_running_error()),
        },
    }
}

fn is_canceled_session_error(error: &AppError) -> bool {
    error.code == TRANSFER_SESSION_CANCELED_CODE
}

impl TransferService {
    pub(super) fn register_session_control(&self, session_id: &str) {
        let (paused_tx, _) = watch::channel(false);
        write_lock(self.session_controls.as_ref(), "session_controls").insert(
            session_id.to_string(),
            RuntimeSessionControl {
                paused_tx,
                canceled: Arc::new(AtomicBool::new(false)),
            },
        );
    }

    pub(super) fn unregister_session_control(&self, session_id: &str) {
        write_lock(self.session_controls.as_ref(), "session_controls").remove(session_id);
        write_lock(self.session_last_emit_ms.as_ref(), "session_last_emit_ms").remove(session_id);
    }

    pub fn pause_session(&self, session_id: &str) -> AppResult<()> {
        self.apply_session_control_action(session_id, SessionControlOperation::Pause)
    }

    pub fn resume_session(&self, session_id: &str) -> AppResult<()> {
        self.apply_session_control_action(session_id, SessionControlOperation::Resume)
    }

    pub fn cancel_session(&self, session_id: &str) -> AppResult<()> {
        self.apply_session_control_action(session_id, SessionControlOperation::Cancel)
    }

    fn read_session_control(&self, session_id: &str) -> Option<RuntimeSessionControl> {
        read_lock(self.session_controls.as_ref(), "session_controls")
            .get(session_id)
            .map(|value| RuntimeSessionControl {
                paused_tx: value.paused_tx.clone(),
                canceled: value.canceled.clone(),
            })
    }

    fn apply_session_control_action(
        &self,
        session_id: &str,
        operation: SessionControlOperation,
    ) -> AppResult<()> {
        let control = self
            .read_session_control(session_id)
            .ok_or_else(|| operation.not_running_error())?;
        let mut session = ensure_session_exists(&self.db_pool, session_id)?;
        let transition =
            resolve_session_control_transition(session.status, operation, now_millis())?;
        session.status = transition.status;
        if let Some(finished_at) = transition.finished_at {
            session.finished_at = Some(finished_at);
        }

        match operation {
            SessionControlOperation::Pause => {
                let _ = control.paused_tx.send(true);
            }
            SessionControlOperation::Resume => {
                let _ = control.paused_tx.send(false);
            }
            SessionControlOperation::Cancel => {
                control.canceled.store(true, Ordering::Relaxed);
                let _ = control.paused_tx.send(false);
            }
        }

        insert_session(&self.db_pool, &session)?;
        self.maybe_emit_session_snapshot(&session, None, 0, None, true, None, None, None, None);
        Ok(())
    }

    pub(super) fn is_session_canceled(&self, session_id: &str) -> bool {
        self.read_session_control(session_id)
            .map(|value| value.canceled.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    pub(super) async fn wait_if_paused(&self, session_id: &str) {
        loop {
            let Some(control) = self.read_session_control(session_id) else {
                return;
            };
            if !*control.paused_tx.borrow() {
                return;
            }

            let mut paused_rx = control.paused_tx.subscribe();
            if !*paused_rx.borrow() {
                return;
            }

            if paused_rx.changed().await.is_err() {
                return;
            }
        }
    }

    pub(super) async fn update_session_failure(
        &self,
        session_id: &str,
        error: &AppError,
    ) -> AppResult<()> {
        let canceled = is_canceled_session_error(error);
        let mut session = self
            .blocking_ensure_session_exists(session_id.to_string())
            .await?;
        Self::mark_session_terminal_state(
            &mut session,
            if canceled {
                TransferStatus::Canceled
            } else {
                TransferStatus::Failed
            },
            if canceled {
                None
            } else {
                Some(error.code.clone())
            },
            if canceled {
                None
            } else {
                Some(error.message.clone())
            },
        );
        let history_reason = if canceled {
            TransferHistorySyncReason::SessionCanceled
        } else {
            TransferHistorySyncReason::SessionFailed
        };
        self.persist_terminal_session_state(
            &session,
            TerminalPersistOptions::new(0).with_history_reason(Some(history_reason)),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_session_control_transition_should_reject_resume_after_cancel() {
        let result = resolve_session_control_transition(
            TransferStatus::Canceled,
            SessionControlOperation::Resume,
            42,
        );
        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_session_not_running");
    }

    #[test]
    fn resolve_session_control_transition_should_mark_cancel_with_finished_at() {
        let transition = resolve_session_control_transition(
            TransferStatus::Running,
            SessionControlOperation::Cancel,
            123,
        )
        .expect("cancel should be allowed for running sessions");

        assert_eq!(transition.status, TransferStatus::Canceled);
        assert_eq!(transition.finished_at, Some(123));
    }

    #[test]
    fn is_canceled_session_error_should_match_transfer_session_canceled_code() {
        assert!(is_canceled_session_error(&AppError::new(
            TRANSFER_SESSION_CANCELED_CODE,
            "canceled",
        )));
        assert!(!is_canceled_session_error(&AppError::new(
            "transfer_session_failed",
            "failed",
        )));
    }
}

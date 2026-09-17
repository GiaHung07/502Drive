//! Telegram user interactive session state machine (FSM).
//!
//! Replaces legacy reply marker matching with a persistent, TTL-bounded FSM.
//! Flow progression:
//! `IDLE → WAIT_SOURCE → INSPECTED → CONFIRM_CLONE | CONFIRM_WATCH | WAIT_DESTINATION`.
//! Commands always take precedence over active sessions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    engine::copy::CloneSourceInspect,
    state::{
        db::{Database, now_ms},
        repo::{self, TelegramSession},
    },
};

pub const DEFAULT_SESSION_TTL_MS: i64 = 10 * 60 * 1000; // 10 minutes

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionFlow {
    Clone,
    CloneHere,
    Watch,
    SetDestination,
    Inspect,
    Prompt(String),
}

impl SessionFlow {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Clone => "clone",
            Self::CloneHere => "clone_here",
            Self::Watch => "watch",
            Self::SetDestination => "set_destination",
            Self::Inspect => "inspect",
            Self::Prompt(action) => action.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "clone" => Self::Clone,
            "clone_here" => Self::CloneHere,
            "watch" => Self::Watch,
            "set_destination" => Self::SetDestination,
            "inspect" => Self::Inspect,
            other => Self::Prompt(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStep {
    WaitSource,
    WaitDestination,
    Inspected,
    WaitInput,
}

impl SessionStep {
    pub fn as_str(&self) -> &str {
        match self {
            Self::WaitSource => "wait_source",
            Self::WaitDestination => "wait_destination",
            Self::Inspected => "inspected",
            Self::WaitInput => "wait_input",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "wait_source" => Self::WaitSource,
            "wait_destination" => Self::WaitDestination,
            "inspected" => Self::Inspected,
            _ => Self::WaitInput,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectSessionPayload {
    pub inspect: CloneSourceInspect,
    pub source_input: String,
    pub message_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DestinationSessionPayload {
    pub return_to_inspect: Option<InspectSessionPayload>,
}

pub async fn get_session(
    db: &Database,
    user_id: i64,
    chat_id: i64,
) -> anyhow::Result<Option<TelegramSession>> {
    repo::get_telegram_session(db, user_id, chat_id).await
}

pub async fn start_session(
    db: &Database,
    user_id: i64,
    chat_id: i64,
    flow: SessionFlow,
    step: SessionStep,
    payload_json: String,
) -> anyhow::Result<TelegramSession> {
    let now = now_ms();
    let session = TelegramSession {
        id: Uuid::new_v4().to_string(),
        user_id,
        chat_id,
        flow: flow.as_str().to_string(),
        step: step.as_str().to_string(),
        payload_json,
        created_at_ms: now,
        expires_at_ms: now + DEFAULT_SESSION_TTL_MS,
    };
    repo::upsert_telegram_session(db, session.clone()).await?;
    Ok(session)
}

pub async fn clear_session(db: &Database, user_id: i64, chat_id: i64) -> anyhow::Result<()> {
    repo::delete_telegram_session(db, user_id, chat_id).await
}

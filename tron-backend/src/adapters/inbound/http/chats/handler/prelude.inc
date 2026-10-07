use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
    io::Seek,
    sync::Arc,
    time::Duration,
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::{
        agent::{AgentInvocationService, generate_agent_run_code},
        chat::{
            ArchivedChatSessionPage, ArchivedChatSessionQuery, BatchDeleteArchivedSessionItem,
            BatchDeleteArchivedSessionsCommand, BatchDeleteArchivedSessionsResult, ChatSendError,
            ChatSessionAttachmentUploadCommand, ChatSessionQuery, CreateChatSessionCommand,
            SendChatMessageCommand, SteerChatMessageCommand, UpdateChatSessionCommand,
        },
    },
    domain::{
        agent::{AgentRuntimeType, ModelStreamEvent},
        chat::ChatError,
    },
    response::{ApiResponse, EmptyData},
    shared::auth_holder::AuthHolder,
};
use axum::{
    Json,
    body::Body,
    extract::{
        DefaultBodyLimit, Multipart, Path, Query, State, multipart::MultipartRejection,
        rejection::JsonRejection,
    },
    http::{
        HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::error_mapper::{
    chat_error_response, chat_send_error_response, chat_steer_error_response,
    steer_chat_message_request_rejection_response,
};
use super::request::{AnswerChatUserInputRequest, SteerChatMessageRequest};
use super::response::{ChatSteerResultResponse, ChatUserInputRequestResponse};
use super::stream_task::{ChatStreamForwardingGate, ChatStreamLifecycleGuard};
mod shared;
use shared::{ChatSsePayload, ChatStreamCompletion};
pub(crate) use shared::{
    chat_send_error_code, chat_send_error_message, normalized_required, prepare_send_model,
    protocol_event_belongs_to_run, resolve_chat_agent_runtime,
};

const CHAT_STREAM_POLL_INTERVAL_MS: u64 = 25;
const MAX_ARCHIVED_SESSION_DELETE_BATCH: usize = 100;
const ARCHIVED_SESSION_DELETE_WORKERS: usize = 4;

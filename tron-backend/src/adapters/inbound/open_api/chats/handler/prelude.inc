use std::{collections::BTreeSet, convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use serde_json::{Value, json};

use crate::{
    adapters::inbound::http::{
        agents::error_mapper::agent_agui_error_response,
        auth::RequestAuthContext,
        chats::error_mapper::{
            chat_error_response, chat_send_error_response,
            steer_chat_message_request_rejection_response,
        },
        chats::handler::{
            chat_send_error_code, chat_send_error_message, ensure_no_active_run,
            ensure_no_pending_user_input, normalized_required, prepare_send_model,
            protocol_event_belongs_to_run, resolve_chat_agent_runtime,
            run_started_with_supports_steer, uses_system_live_stream,
        },
        chats::stream_task::{ChatStreamForwardingGate, ChatStreamLifecycleGuard},
    },
    app::AppState,
    application::{
        agent::{AgentAguiStreamService, AgentInvocationService, generate_agent_run_code},
        chat::ChatSessionQuery,
    },
    domain::{
        agent::{AgentRuntimeType, ModelStreamEvent},
        chat::ChatError,
    },
    response::ApiResponse,
    shared::auth_holder::AuthHolder,
};

use super::{
    request::{
        OpenApiChatSessionQuery, OpenApiCreateChatSessionRequest, OpenApiSendChatMessageRequest,
        OpenApiSteerChatMessageRequest, OpenApiUpdateChatSessionRequest,
    },
    response::{
        OpenApiAguiEventSnapshotResponse, OpenApiAguiProtocolEventResponse,
        OpenApiChatMessageResponse, OpenApiChatSendResultResponse, OpenApiChatSessionGroupResponse,
        OpenApiChatSessionResponse, OpenApiChatSteerResultResponse,
    },
};

const CHAT_STREAM_POLL_INTERVAL_MS: u64 = 25;
const CHAT_STREAM_DRAIN_AFTER_COMPLETION_TICKS: u16 = 80;

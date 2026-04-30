//! ACP server dispatch — receives [`AcpRequest`]s from the wire,
//! authenticates the harness, and routes the four advertised
//! capability methods to a user-supplied [`AcpService`].
use crate::auth::{verify, AuthScheme, AuthToken};
use crate::harness::{Harness, HarnessCapabilities};
use crate::wire::{AcpError, AcpRequest, AcpResponse};
use argo_types::{ArgoResult, SessionId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// `session/send_message` params.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendMessageRequest {
    /// Target session.
    pub session: SessionId,
    /// Message body (text).
    pub text: String,
}

/// One streaming chunk emitted as a server `session/stream_chunk`
/// notification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamChunk {
    /// Session the chunk belongs to.
    pub session: SessionId,
    /// Chunk text.
    pub text: String,
    /// `true` for the final chunk in the stream.
    pub done: bool,
}

/// `session/list` response payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSessionsResponse {
    /// Session ids known to the server.
    pub sessions: Vec<SessionId>,
}

/// `session/fork` params.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForkRequest {
    /// Session to fork from.
    pub source: SessionId,
}

/// User-supplied service that backs the four advertised methods.
///
/// The shipped [`AcpServer`] is just a dispatcher; this is where
/// real Argo state (sessions, gateway, capability checks) lives.
#[async_trait]
pub trait AcpService: Send + Sync {
    /// Handle `session/send_message`.
    async fn send_message(&self, req: SendMessageRequest) -> ArgoResult<()>;
    /// Handle `session/list`.
    async fn list_sessions(&self) -> ArgoResult<ListSessionsResponse>;
    /// Handle `session/fork`. Returns the new session id.
    async fn fork_session(&self, req: ForkRequest) -> ArgoResult<SessionId>;
}

/// JSON-RPC dispatcher in front of an [`AcpService`].
///
/// ACP runs JSON-RPC 2.0 over stdio; the actual stdio loop lives
/// with the harness integration. This struct's job is to take a
/// parsed [`AcpRequest`] + harness auth token and produce the
/// corresponding [`AcpResponse`].
pub struct AcpServer<S: AcpService> {
    /// The connected harness.
    pub harness: Harness,
    /// Auth scheme expected from this harness.
    pub auth_scheme: AuthScheme,
    /// Capabilities advertised at handshake.
    pub capabilities: HarnessCapabilities,
    /// Backing service.
    pub service: S,
}

impl<S: AcpService> AcpServer<S> {
    /// Build a server with default capabilities for `harness`.
    pub fn new(harness: Harness, auth_scheme: AuthScheme, service: S) -> Self {
        Self {
            harness,
            auth_scheme,
            capabilities: HarnessCapabilities::default_for(harness),
            service,
        }
    }

    /// Dispatch one request. `auth_token` is the harness's claimed
    /// credential; it is verified against `self.auth_scheme` before
    /// any method is called.
    pub async fn dispatch(&self, req: AcpRequest, auth_token: &AuthToken) -> AcpResponse {
        if let Err(e) = verify(self.auth_scheme, auth_token) {
            return AcpResponse::err(req.id, AcpError::unauthenticated(e.to_string()));
        }
        self.dispatch_authenticated(req).await
    }

    async fn dispatch_authenticated(&self, req: AcpRequest) -> AcpResponse {
        match req.method.as_str() {
            "session/send_message" => self.handle_send_message(req).await,
            "session/list" => self.handle_list(req).await,
            "session/fork" => self.handle_fork(req).await,
            other => AcpResponse::err(req.id, AcpError::method_not_found(other)),
        }
    }

    async fn handle_send_message(&self, req: AcpRequest) -> AcpResponse {
        if !self.capabilities.send_message {
            return AcpResponse::err(req.id, AcpError::permission_denied("send_message"));
        }
        let id = req.id.clone();
        let parsed: Result<SendMessageRequest, _> = req
            .params
            .ok_or_else(|| serde::de::Error::custom("missing params"))
            .and_then(serde_json::from_value);
        match parsed {
            Ok(p) => match self.service.send_message(p).await {
                Ok(()) => AcpResponse::ok(id, serde_json::json!({"accepted": true})),
                Err(e) => AcpResponse::err(id, AcpError::invalid_params(e.to_string())),
            },
            Err(e) => AcpResponse::err(id, AcpError::invalid_params(e.to_string())),
        }
    }

    async fn handle_list(&self, req: AcpRequest) -> AcpResponse {
        if !self.capabilities.list_sessions {
            return AcpResponse::err(req.id, AcpError::permission_denied("list_sessions"));
        }
        match self.service.list_sessions().await {
            Ok(out) => AcpResponse::ok(req.id, serde_json::to_value(out).unwrap_or_default()),
            Err(e) => AcpResponse::err(req.id, AcpError::invalid_params(e.to_string())),
        }
    }

    async fn handle_fork(&self, req: AcpRequest) -> AcpResponse {
        if !self.capabilities.fork_session {
            return AcpResponse::err(req.id, AcpError::permission_denied("fork_session"));
        }
        let id = req.id.clone();
        let parsed: Result<ForkRequest, _> = req
            .params
            .ok_or_else(|| serde::de::Error::custom("missing params"))
            .and_then(serde_json::from_value);
        match parsed {
            Ok(p) => match self.service.fork_session(p).await {
                Ok(new) => AcpResponse::ok(id, serde_json::json!({"session": new})),
                Err(e) => AcpResponse::err(id, AcpError::invalid_params(e.to_string())),
            },
            Err(e) => AcpResponse::err(id, AcpError::invalid_params(e.to_string())),
        }
    }
}

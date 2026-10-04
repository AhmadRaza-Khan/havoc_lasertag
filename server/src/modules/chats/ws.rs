//! `GET /api/player/chats/ws`: one socket per app session, carrying messages, typing,
//! read receipts and presence. See `events.rs` for the protocol.

use std::time::Duration;

use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, header::AUTHORIZATION},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use uuid::Uuid;

use super::{
    events::{ClientEvent, ServerEvent},
    repository, service,
};
use crate::{
    error::{AppError, AppResult},
    modules::player_auth::authenticate_player,
    state::AppState,
};

/// Server pings this often; clients answer automatically at the protocol level.
const PING_INTERVAL: Duration = Duration::from_secs(25);
/// A connection with no traffic at all (including pongs) for this long is considered dead.
const IDLE_TIMEOUT: Duration = Duration::from_secs(70);
/// Largest frame accepted from a client. Message bodies are capped at 2000 chars anyway.
const MAX_FRAME_BYTES: usize = 16 * 1024;

#[derive(Deserialize)]
pub struct WsQuery {
    /// Alternative to the Authorization header, for clients that can't set headers.
    token: Option<String>,
}

pub async fn connect(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> AppResult<Response> {
    // Authenticate before upgrading so failures are plain HTTP 401/403 responses.
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_owned)
        .or(query.token)
        .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?;
    // Same checks as every player route: player token, account exists, password unchanged.
    let player_id = authenticate_player(&token, &state).await?;

    Ok(ws
        .max_message_size(MAX_FRAME_BYTES)
        .on_upgrade(move |socket| run(socket, state, player_id)))
}

async fn run(socket: WebSocket, state: AppState, player_id: Uuid) {
    let hub = state.chat.clone();
    let conn = hub.register(player_id);
    let conn_id = conn.id;
    tracing::debug!(%player_id, conn_id, "chat socket connected");

    if conn.came_online {
        let last_seen_at = repository::touch_last_seen(&state.db, player_id)
            .await
            .ok()
            .flatten();
        hub.broadcast_except(
            player_id,
            &ServerEvent::Presence {
                player_id,
                online: true,
                last_seen_at,
            },
        );
    }
    hub.send_to_connection(
        player_id,
        conn_id,
        &ServerEvent::Ready {
            player_id,
            online_player_ids: hub.online_players(),
        },
    );

    let (mut sink, mut stream) = socket.split();

    // Writer: queued events + keepalive pings → socket.
    let mut rx = conn.rx;
    let writer = tokio::spawn(async move {
        let mut ping = tokio::time::interval(PING_INTERVAL);
        ping.tick().await; // first tick is immediate
        loop {
            tokio::select! {
                event = rx.recv() => match event {
                    Some(text) => {
                        if sink.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                },
                _ = ping.tick() => {
                    if sink.send(Message::Ping(Default::default())).await.is_err() {
                        break;
                    }
                }
            }
        }
        let _ = sink.close().await;
    });

    // Reader: client events until the socket closes or goes quiet.
    loop {
        let frame = match tokio::time::timeout(IDLE_TIMEOUT, stream.next()).await {
            Ok(Some(Ok(frame))) => frame,
            Ok(Some(Err(e))) => {
                tracing::debug!(%player_id, "chat socket error: {e}");
                break;
            }
            Ok(None) => break,
            Err(_) => {
                tracing::debug!(%player_id, "chat socket idle timeout");
                break;
            }
        };
        match frame {
            Message::Text(text) => handle_event(&state, player_id, conn_id, text.as_str()).await,
            Message::Close(_) => break,
            // Pings are answered by axum; pongs just keep the connection alive.
            _ => {}
        }
    }

    writer.abort();
    if hub.unregister(player_id, conn_id) {
        let last_seen_at = repository::touch_last_seen(&state.db, player_id)
            .await
            .ok()
            .flatten();
        hub.broadcast_except(
            player_id,
            &ServerEvent::Presence {
                player_id,
                online: false,
                last_seen_at,
            },
        );
    }
    tracing::debug!(%player_id, conn_id, "chat socket disconnected");
}

async fn handle_event(state: &AppState, player_id: Uuid, conn_id: u64, text: &str) {
    let reply = |event: ServerEvent| state.chat.send_to_connection(player_id, conn_id, &event);

    let event = match serde_json::from_str::<ClientEvent>(text) {
        Ok(e) => e,
        Err(e) => {
            return reply(ServerEvent::Error {
                message: format!("Invalid event: {e}"),
                client_id: None,
            });
        }
    };

    match event {
        ClientEvent::Send {
            to,
            body,
            client_id,
        } => {
            if let Err(e) =
                service::send_message(state, player_id, to, &body, client_id.clone()).await
            {
                reply(ServerEvent::Error {
                    message: client_message(e),
                    client_id,
                });
            }
        }
        ClientEvent::Typing { to, is_typing } => service::typing(state, player_id, to, is_typing),
        ClientEvent::Read { with } => {
            if let Err(e) = service::mark_read(state, player_id, with).await {
                reply(ServerEvent::Error {
                    message: client_message(e),
                    client_id: None,
                });
            }
        }
        ClientEvent::Ping => reply(ServerEvent::Pong),
    }
}

/// User-facing text for an error; internal details are logged, not sent.
fn client_message(e: AppError) -> String {
    match e {
        AppError::Database(err) => {
            tracing::error!("chat database error: {err}");
            "Something went wrong, please try again".into()
        }
        AppError::Internal(err) => {
            tracing::error!("chat internal error: {err}");
            "Something went wrong, please try again".into()
        }
        other => other.to_string(),
    }
}

use crate::error::AppError;
use crate::utils::jwt::decode_jwt;
use crate::websocket::hub::NotificationHub;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, WebSocketUpgrade,
    },
    response::IntoResponse,
    Extension,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    Extension(hub): Extension<NotificationHub>,
) -> Result<impl IntoResponse, AppError> {
    let claims = decode_jwt(&query.token).map_err(|_| AppError::Unauthorized)?;
    let user_id: i32 = claims.sub.parse().map_err(|_| AppError::Unauthorized)?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, user_id, hub)))
}

async fn handle_socket(socket: WebSocket, user_id: i32, hub: NotificationHub) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut rx = hub.subscribe(user_id);

    tracing::info!("WebSocket connected for user {}", user_id);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    tracing::info!("WebSocket disconnected for user {}", user_id);
}

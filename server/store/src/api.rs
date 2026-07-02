//! REST API handlers

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde_json::json;

use crate::auth;

/// Autentica a requisição (SEC-09); retorna o peer autenticado ou uma
/// resposta 401 pronta.
fn authenticate(http_req: &HttpRequest) -> Result<String, HttpResponse> {
    auth::verify_request(http_req).map_err(|e| {
        tracing::warn!("🚫 Unauthorized store request: {}", e.0);
        HttpResponse::Unauthorized().json(json!({ "error": e.0 }))
    })
}

use crate::database::Database;
use crate::models::{
    DeleteMessagesRequest, DeleteMessagesResponse, HealthResponse, OfflineMessageDto,
    RetrieveMessagesRequest, RetrieveMessagesResponse, StoreMessageRequest, StoreMessageResponse,
};
use crate::push_notifier::PushNotifier;
use crate::redis_client::RedisClient;

/// Store a new offline message
///
/// POST /api/store
pub async fn store_message(
    http_req: HttpRequest,
    db: web::Data<Database>,
    redis: web::Data<RedisClient>,
    push: web::Data<PushNotifier>,
    req: web::Json<StoreMessageRequest>,
) -> impl Responder {
    // SEC-09: só o próprio remetente pode armazenar em seu nome
    let auth_peer = match authenticate(&http_req) {
        Ok(peer) => peer,
        Err(resp) => return resp,
    };
    if auth_peer != req.sender_peer_id {
        return HttpResponse::Forbidden().json(json!({
            "error": "sender_peer_id does not match authenticated peer"
        }));
    }

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": e
        }));
    }

    // Store in database
    match db.store_message(&req).await {
        Ok((id, message_id)) => {
            // Get message details for response
            let expires_at = chrono::Utc::now() + chrono::Duration::days(14);

            // Increment stats
            let _ = db.increment_stats("store").await;

            // Notify recipient via Redis (if online)
            let _ = redis
                .publish_message_notification(&req.recipient_peer_id)
                .await;

            // PSH-02: acordar o destinatário via push (conteúdo nunca vai no push)
            push.notify_offline_message(&req.recipient_peer_id, &req.sender_peer_id);

            HttpResponse::Created().json(StoreMessageResponse {
                id,
                message_id,
                created_at: chrono::Utc::now(),
                expires_at,
            })
        }
        Err(e) => {
            tracing::error!("Failed to store message: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to store message"
            }))
        }
    }
}

/// Retrieve pending messages for a recipient
///
/// GET /api/store?peer_id={peer_id}&limit={limit}
pub async fn retrieve_messages(
    http_req: HttpRequest,
    db: web::Data<Database>,
    query: web::Query<RetrieveMessagesRequest>,
) -> impl Responder {
    // SEC-09: um peer só lê as PRÓPRIAS mensagens pendentes
    let auth_peer = match authenticate(&http_req) {
        Ok(peer) => peer,
        Err(resp) => return resp,
    };
    if auth_peer != query.peer_id {
        return HttpResponse::Forbidden().json(json!({
            "error": "peer_id does not match authenticated peer"
        }));
    }

    if query.peer_id.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "peer_id is required"
        }));
    }

    match db.retrieve_messages(&query.peer_id, query.limit).await {
        Ok(messages) => {
            let total = messages.len() as i64;
            let messages_dto: Vec<OfflineMessageDto> =
                messages.into_iter().map(|m| m.into()).collect();

            HttpResponse::Ok().json(RetrieveMessagesResponse {
                messages: messages_dto,
                total,
            })
        }
        Err(e) => {
            tracing::error!("Failed to retrieve messages: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to retrieve messages"
            }))
        }
    }
}

/// Delete (acknowledge) messages
///
/// DELETE /api/store
pub async fn delete_messages(
    http_req: HttpRequest,
    db: web::Data<Database>,
    req: web::Json<DeleteMessagesRequest>,
) -> impl Responder {
    // SEC-09: ack restrito a mensagens endereçadas ao peer autenticado
    let auth_peer = match authenticate(&http_req) {
        Ok(peer) => peer,
        Err(resp) => return resp,
    };

    if req.message_ids.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "message_ids is required"
        }));
    }

    match db.delete_messages(&req.message_ids, &auth_peer).await {
        Ok(deleted_count) => HttpResponse::Ok().json(DeleteMessagesResponse { deleted_count }),
        Err(e) => {
            tracing::error!("Failed to delete messages: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to delete messages"
            }))
        }
    }
}

/// Health check endpoint
///
/// GET /health
pub async fn health_check(
    db: web::Data<Database>,
    redis: web::Data<RedisClient>,
) -> impl Responder {
    // Check database
    let db_status = match db.health_check().await {
        Ok(_) => "healthy".to_string(),
        Err(e) => {
            tracing::error!("Database health check failed: {:?}", e);
            "unhealthy".to_string()
        }
    };

    // Check Redis
    let redis_status = match redis.health_check().await {
        Ok(_) => "healthy".to_string(),
        Err(e) => {
            tracing::error!("Redis health check failed: {:?}", e);
            "unhealthy".to_string()
        }
    };

    // Count pending messages
    let pending_messages = db.count_pending_messages().await.unwrap_or(0);

    let overall_status = if db_status == "healthy" && redis_status == "healthy" {
        "healthy"
    } else {
        "degraded"
    };

    HttpResponse::Ok().json(HealthResponse {
        status: overall_status.to_string(),
        database: db_status,
        redis: redis_status,
        pending_messages,
    })
}

/// Get statistics
///
/// GET /api/stats
pub async fn get_stats(db: web::Data<Database>) -> impl Responder {
    match db.count_pending_messages().await {
        Ok(pending) => HttpResponse::Ok().json(json!({
            "pending_messages": pending,
            "timestamp": chrono::Utc::now()
        })),
        Err(e) => {
            tracing::error!("Failed to get stats: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get stats"
            }))
        }
    }
}

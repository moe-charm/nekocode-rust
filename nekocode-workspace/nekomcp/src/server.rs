//! MCP server implementation for NekoCode

use anyhow::{anyhow, Result};
use axum::{
    extract::{Query, State}, 
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use nekocode_core::{
    session::SessionManager,
    types::*,
};
use chrono;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

/// MCP server state
#[derive(Clone)]
pub struct McpServerState {
    pub session_manager: Arc<RwLock<SessionManager>>,
}

/// MCP request types
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub path: PathBuf,
    pub language: Option<String>,
    pub stats_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SessionCreateRequest {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct SessionUpdateRequest {
    pub session_id: String,
    pub verbose: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SessionStatsRequest {
    pub session_id: String,
}

/// MCP response types
#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub success: bool,
    pub data: Option<AnalysisResult>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub success: bool,
    pub session_id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CapabilitiesResponse {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub supported_languages: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime: u64,
}

impl McpServerState {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(RwLock::new(SessionManager::new().expect("Failed to create SessionManager"))),
        }
    }
}

/// Create MCP server router
pub fn create_router(state: McpServerState, enable_cors: bool) -> Router {
    let mut router = Router::new()
        .route("/health", get(health_handler))
        .route("/capabilities", get(capabilities_handler))
        .route("/analyze", post(analyze_handler))
        .route("/session/create", post(session_create_handler))
        .route("/session/update", post(session_update_handler))
        .route("/session/stats", get(session_stats_handler))
        .route("/session/list", get(session_list_handler))
        .with_state(state);

    if enable_cors {
        router = router.layer(CorsLayer::permissive());
    }

    router
}

/// Health check handler
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: crate::VERSION.to_string(),
        uptime: 0, // TODO: Track actual uptime
    })
}

/// Capabilities handler
async fn capabilities_handler() -> Json<CapabilitiesResponse> {
    Json(CapabilitiesResponse {
        name: "NekoCode MCP Server".to_string(),
        version: crate::VERSION.to_string(),
        capabilities: vec![
            "analyze".to_string(),
            "session_management".to_string(),
            "incremental_analysis".to_string(),
            "ast_operations".to_string(),
            "refactoring".to_string(),
            "impact_analysis".to_string(),
        ],
        supported_languages: vec![
            "javascript".to_string(),
            "typescript".to_string(),
            "python".to_string(),
            "cpp".to_string(),
            "c".to_string(),
            "csharp".to_string(),
            "go".to_string(),
            "rust".to_string(),
        ],
    })
}

/// Analyze handler
async fn analyze_handler(
    State(state): State<McpServerState>,
    Json(request): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, StatusCode> {
    log::info!("Analyzing path: {:?}", request.path);

    let mut session_manager = state.session_manager.write().await;
    
    match session_manager.create_session(request.path.clone()) {
        Ok(_session_id) => {
            // For now, return a mock analysis result
            // TODO: Integrate with actual analysis engine
            let mock_file_info = FileInfo {
                name: request.path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                path: request.path.clone(),
                language: Language::JavaScript, // Default to JavaScript
                size_bytes: 1000,
                total_lines: 100,
                code_lines: 80,
                comment_lines: 10,
                empty_lines: 10,
                code_ratio: 0.8,
                analyzed_at: chrono::Utc::now(),
                hash: Some("mock_hash".to_string()),
                metadata: std::collections::HashMap::new(),
            };
            
            let mock_result = AnalysisResult {
                file_info: mock_file_info,
                symbols: vec![],
                functions: vec![],
                classes: vec![],
                imports: vec![],
                exports: vec![],
                dependencies: vec![],
                metrics: CodeMetrics {
                    lines_of_code: 100,
                    lines_with_comments: 10,
                    blank_lines: 10,
                    cyclomatic_complexity: Some(5),
                    halstead_volume: Some(200.0),
                    maintainability_index: Some(80.0),
                },
                errors: vec![],
            };

            let response_data = if request.stats_only.unwrap_or(false) {
                // Return only basic info and metrics for stats_only
                AnalysisResult {
                    file_info: mock_result.file_info.clone(),
                    symbols: vec![], // Empty symbols for stats_only
                    functions: vec![], // Empty functions for stats_only
                    classes: vec![], // Empty classes for stats_only
                    imports: vec![], // Empty imports for stats_only  
                    exports: vec![], // Empty exports for stats_only
                    dependencies: vec![], // Empty dependencies for stats_only
                    metrics: mock_result.metrics.clone(),
                    errors: vec![],
                }
            } else {
                mock_result
            };

            Ok(Json(AnalyzeResponse {
                success: true,
                data: Some(response_data),
                error: None,
            }))
        }
        Err(e) => Ok(Json(AnalyzeResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Session create handler
async fn session_create_handler(
    State(state): State<McpServerState>,
    Json(request): Json<SessionCreateRequest>,
) -> Result<Json<SessionResponse>, StatusCode> {
    log::info!("Creating session for path: {:?}", request.path);

    let mut session_manager = state.session_manager.write().await;
    
    match session_manager.create_session(request.path) {
        Ok(session_id) => Ok(Json(SessionResponse {
            success: true,
            session_id: Some(session_id),
            data: None,
            error: None,
        })),
        Err(e) => Ok(Json(SessionResponse {
            success: false,
            session_id: None,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Session update handler
async fn session_update_handler(
    State(state): State<McpServerState>,
    Json(request): Json<SessionUpdateRequest>,
) -> Result<Json<SessionResponse>, StatusCode> {
    log::info!("Updating session: {}", request.session_id);

    let mut session_manager = state.session_manager.write().await;
    
    match session_manager.get_session_mut(&request.session_id) {
        Ok(session) => {
            // Touch the session to update access time
            session.touch();
            
            // For now, return basic session info
            // TODO: Implement actual incremental update logic
            let update_result = serde_json::json!({
                "session_id": request.session_id,
                "path": session.path(),
                "last_accessed": session.info.last_accessed,
                "last_modified": session.info.last_modified,
                "file_count": session.info.file_count,
                "total_lines": session.info.total_lines,
            });
            
            Ok(Json(SessionResponse {
                success: true,
                session_id: Some(request.session_id),
                data: Some(update_result),
                error: None,
            }))
        }
        Err(e) => Ok(Json(SessionResponse {
            success: false,
            session_id: Some(request.session_id),
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Session stats handler
async fn session_stats_handler(
    State(state): State<McpServerState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let session_id = params.get("session_id")
        .ok_or(StatusCode::BAD_REQUEST)?;

    log::info!("Getting stats for session: {}", session_id);

    let mut session_manager = state.session_manager.write().await;
    
    match session_manager.get_session(session_id) {
        Ok(session) => {
            let stats = serde_json::json!({
                "session_id": session_id,
                "path": session.path(),
                "created_at": session.info.created_at,
                "last_accessed": session.info.last_accessed,
                "last_modified": session.info.last_modified,
                "file_count": session.info.file_count,
                "total_lines": session.info.total_lines,
                "languages": session.info.languages,
                "version": session.info.version,
                "is_dirty": session.info.is_dirty,
            });
            
            Ok(Json(SessionResponse {
                success: true,
                session_id: Some(session_id.clone()),
                data: Some(stats),
                error: None,
            }))
        }
        Err(e) => Ok(Json(SessionResponse {
            success: false,
            session_id: Some(session_id.clone()),
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Session list handler
async fn session_list_handler(
    State(state): State<McpServerState>,
) -> Json<SessionResponse> {
    log::info!("Listing all sessions");

    let session_manager = state.session_manager.read().await;
    
    match session_manager.list_sessions() {
        Ok(sessions) => Json(SessionResponse {
            success: true,
            session_id: None,
            data: Some(serde_json::to_value(sessions).unwrap_or_default()),
            error: None,
        }),
        Err(e) => Json(SessionResponse {
            success: false,
            session_id: None,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Start the MCP server
pub async fn start_server(host: &str, port: u16, enable_cors: bool) -> Result<()> {
    let state = McpServerState::new();
    let app = create_router(state, enable_cors);

    let addr = format!("{}:{}", host, port);
    log::info!("Starting MCP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow!("Failed to bind to {}: {}", addr, e))?;

    log::info!("NekoMCP server listening on {}", addr);
    
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow!("Server error: {}", e))?;

    Ok(())
}
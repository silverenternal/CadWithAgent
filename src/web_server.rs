//! Web API server for CadAgent
//!
//! This module provides a REST API for the Web UI to interact with CadAgent.
//! It uses Axum as the web framework and supports:
//! - File upload/download
//! - Chat with AI assistant
//! - Tool execution
//! - Constraint solving
//! - Model export

use axum::{
    extract::{Multipart, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::prelude::*;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: ServerConfig,
    pub primitives: Arc<tokio::sync::RwLock<Vec<Primitive>>>,
    pub conversation: Arc<tokio::sync::RwLock<Vec<ChatMessage>>>,
}

/// Server configuration
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub upload_dir: PathBuf,
    pub export_dir: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            upload_dir: PathBuf::from("./uploads"),
            export_dir: PathBuf::from("./exports"),
        }
    }
}

/// Chat request
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
    pub primitives: Option<Vec<Primitive>>,
}

/// Chat response
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub conversation_id: String,
    pub actions: Vec<Action>,
}

/// Action for the frontend to execute
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    CreatePrimitive { primitive: Primitive },
    ModifyPrimitive { id: String, changes: serde_json::Value },
    DeletePrimitive { id: String },
    ApplyConstraint { constraint: serde_json::Value },
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Analysis result
#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    pub primitives: Vec<Primitive>,
    pub constraints: Vec<ConstraintInfo>,
    pub features: Vec<FeatureInfo>,
}

#[derive(Debug, Serialize)]
pub struct ConstraintInfo {
    pub id: String,
    pub r#type: String,
    pub entities: Vec<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct FeatureInfo {
    pub id: String,
    pub name: String,
    pub r#type: String,
}

/// Tool execution request
#[derive(Debug, Deserialize)]
pub struct ToolRequest {
    pub tool: String,
    pub parameters: serde_json::Value,
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(handle_chat))
        .route("/upload", post(handle_upload))
        .route("/export/:format", post(handle_export))
        .route("/tools", get(list_tools))
        .route("/tools/execute", post(execute_tool))
        .route("/constraints/apply", post(apply_constraints))
        .route("/constraints/solve", post(solve_constraints))
        .layer(cors)
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: crate::VERSION.to_string(),
    })
}

/// Chat endpoint
async fn handle_chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    info!("Received chat message: {}", req.message);

    // Store user message
    {
        let mut conv = state.conversation.write().await;
        conv.push(ChatMessage {
            role: "user".to_string(),
            content: req.message.clone(),
        });
    }

    // Update primitives if provided
    if let Some(primitives) = req.primitives {
        let mut prim = state.primitives.write().await;
        *prim = primitives;
    }

    // TODO: Integrate with LLM reasoning engine
    // For now, return a simple response
    let response = format!(
        "I understand you want to: \"{}\". \n\n\
        I can help you with:\n\
        1. Creating geometry\n\
        2. Applying constraints\n\
        3. Parametric modeling\n\n\
        What would you like to do first?",
        req.message
    );

    // Store assistant response
    {
        let mut conv = state.conversation.write().await;
        conv.push(ChatMessage {
            role: "assistant".to_string(),
            content: response.clone(),
        });
    }

    Ok(Json(ChatResponse {
        response,
        conversation_id: req.conversation_id.unwrap_or_else(|| "default".to_string()),
        actions: vec![],
    }))
}

/// File upload endpoint
async fn handle_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<AnalysisResult>, ApiError> {
    info!("Received file upload");

    // Ensure upload directory exists
    tokio::fs::create_dir_all(&state.config.upload_dir).await?;

    let mut primitives = Vec::new();

    // Process uploaded file
    while let Some(field) = multipart.next_field().await? {
        let file_name = field.file_name().map(|s| s.to_string());
        let data = field.bytes().await?;

        if let Some(name) = file_name {
            let path = state.config.upload_dir.join(&name);
            tokio::fs::write(&path, &data).await?;

            // Parse based on file extension
            let extension = name.split('.').next_back().unwrap_or("").to_lowercase();
            match extension.as_str() {
                "svg" => {
                    let parsed = SvgParser::parse(&path)?;
                    primitives = parsed.primitives;
                }
                "dxf" => {
                    // TODO: DXF parsing
                    info!("DXF file uploaded: {}", name);
                }
                "step" | "stp" => {
                    // TODO: STEP parsing
                    info!("STEP file uploaded: {}", name);
                }
                "iges" | "igs" => {
                    // TODO: IGES parsing
                    info!("IGES file uploaded: {}", name);
                }
                _ => {
                    return Err(ApiError::UnsupportedFormat(extension));
                }
            }
        }
    }

    // Update state
    {
        let mut prim = state.primitives.write().await;
        *prim = primitives.clone();
    }

    Ok(Json(AnalysisResult {
        primitives,
        constraints: vec![],
        features: vec![],
    }))
}

/// Export endpoint
async fn handle_export(
    State(state): State<AppState>,
    axum::extract::Path(format): axum::extract::Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Exporting model to format: {}", format);

    let primitives = state.primitives.read().await;

    // Ensure export directory exists
    tokio::fs::create_dir_all(&state.config.export_dir).await?;

    match format.as_str() {
        "svg" => {
            let output_path = state.config.export_dir.join("export.svg");
            DxfExporter::export(&primitives, &output_path)?;
            
            let data = tokio::fs::read(&output_path).await?;
            
            Ok((
                [(header::CONTENT_TYPE, "image/svg+xml")],
                data,
            ))
        }
        "dxf" => {
            let output_path = state.config.export_dir.join("export.dxf");
            DxfExporter::export(&primitives, &output_path)?;
            
            let data = tokio::fs::read(&output_path).await?;
            
            Ok((
                [(header::CONTENT_TYPE, "application/dxf")],
                data,
            ))
        }
        _ => Err(ApiError::UnsupportedFormat(format)),
    }
}

/// List available tools
async fn list_tools() -> Json<serde_json::Value> {
    let registry = ToolRegistry::new();
    let tools = registry.list_tools();
    
    Json(serde_json::to_value(tools).unwrap_or_default())
}

/// Execute a tool
async fn execute_tool(
    Json(req): Json<ToolRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Executing tool: {}", req.tool);

    let registry = ToolRegistry::new();
    let result = registry.call(&req.tool, req.parameters)?;

    Ok(Json(result))
}

/// Apply constraints
async fn apply_constraints(
    Json(req): Json<ApplyConstraintsRequest>,
) -> Result<Json<ApplyConstraintsResponse>, ApiError> {
    info!("Applying {} constraints", req.constraints.len());

    // TODO: Implement constraint application
    // For now, return the current primitives
    
    Ok(Json(ApplyConstraintsResponse {
        primitives: vec![],
    }))
}

#[derive(Debug, Deserialize)]
struct ApplyConstraintsRequest {
    constraints: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ApplyConstraintsResponse {
    primitives: Vec<Primitive>,
}

/// Solve constraints
async fn solve_constraints(
    State(state): State<AppState>,
) -> Result<Json<SolveConstraintsResponse>, ApiError> {
    info!("Solving constraints");

    let primitives = state.primitives.read().await.clone();

    // TODO: Implement constraint solving
    
    Ok(Json(SolveConstraintsResponse {
        primitives,
        status: "solved".to_string(),
    }))
}

#[derive(Debug, Serialize)]
struct SolveConstraintsResponse {
    primitives: Vec<Primitive>,
    status: String,
}

/// API error type
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("Geometry error: {0}")]
    Geometry(#[from] crate::error::CadAgentError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Multipart error: {0}")]
    Multipart(#[from] axum::extract::multipart::MultipartError),

    #[error("Tool error: {0}")]
    Tool(#[from] crate::tools::registry::ToolError),

    #[error("DXF export error: {0}")]
    DxfExport(#[from] crate::export::dxf::DxfExportError),

    #[error("SVG parsing error: {0}")]
    SvgParse(#[from] crate::parser::svg::SvgError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ApiError::UnsupportedFormat(_) => StatusCode::BAD_REQUEST,
            ApiError::Geometry(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Multipart(_) => StatusCode::BAD_REQUEST,
            ApiError::Tool(_) => StatusCode::BAD_REQUEST,
            ApiError::DxfExport(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::SvgParse(_) => StatusCode::BAD_REQUEST,
        };

        let body = serde_json::json!({
            "error": self.to_string()
        });

        (status, Json(body)).into_response()
    }
}

/// Start the web server
pub async fn start_server(config: ServerConfig) -> std::io::Result<()> {
    let state = AppState {
        config: config.clone(),
        primitives: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        conversation: Arc::new(tokio::sync::RwLock::new(vec![])),
    };

    let app = create_router(state);

    let addr = format!("{}:{}", config.host, config.port);
    info!("Starting web server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await
}

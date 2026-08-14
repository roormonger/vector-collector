use crate::query::{ContextRequest, FacetsRequest, Filters, SearchRequest};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::Server;
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Vector Collector",
        description = "REST API for ingesting Vector logs and searching them (also exposed via MCP). \
Use a bearer API key: ingest-scoped for `/v1/logs`, query-scoped for `/v1/query/*`.",
        version = "0.1.0"
    ),
    paths(
        crate::routes::healthz,
        crate::routes::readyz,
        crate::routes::ingest_health,
        crate::routes::ingest_logs,
        crate::routes::query_schema,
        crate::routes::query_facets,
        crate::routes::query_search,
        crate::routes::query_event,
        crate::routes::query_context,
    ),
    components(schemas(
        HealthResponse,
        ReadyResponse,
        IngestHealthResponse,
        IngestAccepted,
        ErrorBody,
        Filters,
        SearchRequest,
        FacetsRequest,
        ContextRequest,
    )),
    tags(
        (name = "health", description = "Liveness / readiness"),
        (name = "ingest", description = "Vector HTTP sink ingest"),
        (name = "query", description = "Search / facets / context (query API key)"),
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Query-only spec for Open WebUI Tool Servers (no ingest).
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Vector Collector Query",
        description = "Search logs in Vector Collector with a query-scoped bearer API key \
(from the admin MCP tab). Recommended loop: GET /v1/query/schema → POST /v1/query/facets → \
POST /v1/query/search → POST /v1/query/context. Do not ingest logs through this API.",
        version = "0.1.0"
    ),
    paths(
        crate::routes::query_schema,
        crate::routes::query_facets,
        crate::routes::query_search,
        crate::routes::query_event,
        crate::routes::query_context,
    ),
    components(schemas(
        ErrorBody,
        Filters,
        SearchRequest,
        FacetsRequest,
        ContextRequest,
    )),
    tags(
        (name = "query", description = "Search / facets / context (query API key)"),
    ),
    modifiers(&QuerySecurityAddon)
)]
pub struct QueryApiDoc;

pub fn query_openapi(public_base_url: &str) -> utoipa::openapi::OpenApi {
    let mut spec = QueryApiDoc::openapi();
    spec.servers = Some(vec![Server::new(
        public_base_url.trim_end_matches('/').to_string(),
    )]);
    spec
}

struct SecurityAddon;

struct QuerySecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_ingest",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("lk_…")
                    .description(Some(
                        "Ingest-scoped API key from an agent".to_string(),
                    ))
                    .build(),
            ),
        );
        components.add_security_scheme(
            "bearer_query",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("lk_…")
                    .description(Some(
                        "Query-scoped API key from the MCP tab".to_string(),
                    ))
                    .build(),
            ),
        );
    }
}

impl Modify for QuerySecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_query",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("lk_…")
                    .description(Some(
                        "Query-scoped API key from the MCP tab".to_string(),
                    ))
                    .build(),
            ),
        );
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ReadyResponse {
    pub ok: bool,
    pub queue_depth: usize,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct IngestHealthResponse {
    pub ok: bool,
    pub key: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct IngestAccepted {
    pub written: usize,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub error: String,
}

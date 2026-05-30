use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

use crate::client::{self, DepError};

pub const SERVICE: &str = "srvcs-vectoradd";
pub const CONCERN: &str = "vectors: component-wise addition";
pub const DEPENDS_ON: &[&str] = &["srvcs-floatadd"];

/// Dependency endpoints, injected as router state so tests can point them at
/// mock services.
#[derive(Clone)]
pub struct Deps {
    pub floatadd_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    /// The first vector: a JSON array of numbers.
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    /// The second vector: a JSON array of numbers. Must match `a` in length.
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct VectorAddResponse {
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
    /// The component-wise sum, a JSON array of `f64` numbers.
    #[schema(value_type = Object)]
    pub result: Vec<f64>,
}

fn ok(a: Vec<Value>, b: Vec<Value>, result: Vec<f64>) -> Response {
    (
        StatusCode::OK,
        Json(json!({ "a": a, "b": b, "result": result })),
    )
        .into_response()
}

fn degraded(dependency: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "dependency unavailable", "dependency": dependency })),
    )
        .into_response()
}

fn forward(status: u16, body: Value) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (code, Json(body)).into_response()
}

/// A reachable dependency answered `200` but its body lacked a numeric
/// `result`. That is a contract violation we cannot recover from, so surface a
/// `500` rather than guessing.
fn malformed(dependency: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(
            json!({ "error": "dependency returned a malformed result", "dependency": dependency }),
        ),
    )
        .into_response()
}

/// Length mismatch between the two operand vectors. This is the one piece of
/// validation this orchestrator owns: the vectors must be component-wise
/// alignable, so a mismatch is a `422` we raise ourselves.
fn length_mismatch(a_len: usize, b_len: usize) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({
            "error": "vectors must have equal length",
            "a_len": a_len,
            "b_len": b_len,
        })),
    )
        .into_response()
}

/// Call one dependency at `url` with `body`, mapping its outcome to either the
/// numeric `result` (on `200`) or an early-return `Response` the caller should
/// surface verbatim:
///
/// - unreachable / non-`200`/`422` -> `503` degraded
/// - `422` -> forwarded `422` (the dependency rejected the input)
/// - `200` without a numeric `result` -> `500` malformed
async fn ask(url: &str, body: &Value, dependency: &str) -> Result<f64, Response> {
    match client::call(url, body).await {
        Err(DepError::Unreachable) => Err(degraded(dependency)),
        Ok((200, body)) => match body.get("result").and_then(Value::as_f64) {
            Some(r) => Ok(r),
            None => Err(malformed(dependency)),
        },
        Ok((422, body)) => Err(forward(422, body)),
        Ok(_) => Err(degraded(dependency)),
    }
}

/// `POST /` — component-wise vector addition.
///
/// This service owns the *control flow* but delegates every arithmetic step to
/// `srvcs-floatadd`. The two operand vectors must have equal length (else
/// `422`). For each index `i` it computes `floatadd(a[i], b[i]).result` and
/// pushes it onto the result list.
///
/// Element-level validation is not handled here: this service never calls
/// `srvcs-isnumber` directly. If `floatadd` is unreachable it reports itself
/// degraded (`503`); if `floatadd` rejects a component it forwards the `422`.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = VectorAddResponse),
        (status = 422, description = "vector length mismatch, or a dependency rejected a component (forwarded)"),
        (status = 500, description = "a dependency returned a malformed result"),
        (status = 503, description = "a dependency is unavailable")
    )
)]
pub async fn evaluate(State(deps): State<Deps>, Json(req): Json<EvalRequest>) -> Response {
    let EvalRequest { a, b } = req;

    if a.len() != b.len() {
        return length_mismatch(a.len(), b.len());
    }

    let mut result: Vec<f64> = Vec::with_capacity(a.len());
    for (ai, bi) in a.iter().zip(b.iter()) {
        let r = match ask(
            &deps.floatadd_url,
            &json!({ "a": ai, "b": bi }),
            "srvcs-floatadd",
        )
        .await
        {
            Ok(v) => v,
            Err(resp) => return resp,
        };
        result.push(r);
    }

    ok(a, b, result)
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, VectorAddResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some());
        assert!(root.post.is_some());
    }

    #[tokio::test]
    async fn index_reports_dependency() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-vectoradd");
        assert_eq!(info.concern, "vectors: component-wise addition");
        assert_eq!(info.depends_on, vec!["srvcs-floatadd"]);
    }
}

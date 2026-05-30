use axum::body::Body;
use axum::extract::Json as AxumJson;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_vectoradd::{api::Deps, health, router, telemetry};
use tower::ServiceExt;

const DEAD_URL: &str = "http://127.0.0.1:1";

// --- Computing mocks for every srvcs primitive this family composes over.
//
// Each reads its operands from the request body and returns the *real* answer,
// so the orchestration is genuinely exercised rather than fed a canned value.
// vectoradd only calls floatadd; the rest are provided for completeness of the
// family's contract.

/// `srvcs-floatadd`: reads `{a, b}` -> `{"result": a + b}` (as f64).
async fn spawn_floatadd() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a + b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatsubtract`: reads `{a, b}` -> `{"result": a - b}` (as f64).
#[allow(dead_code)]
async fn spawn_floatsubtract() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a - b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatmultiply`: reads `{a, b}` -> `{"result": a * b}` (as f64).
#[allow(dead_code)]
async fn spawn_floatmultiply() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a * b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatdivide`: reads `{a, b}` -> `{"result": a / b}` (as f64).
#[allow(dead_code)]
async fn spawn_floatdivide() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(1.0);
            Json(json!({ "result": a / b }))
        }),
    );
    serve(app).await
}

/// `srvcs-sqrt`: reads `{value}` -> `{"result": sqrt(value)}` (as f64).
#[allow(dead_code)]
async fn spawn_sqrt() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let value = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": value.sqrt() }))
        }),
    );
    serve(app).await
}

/// `srvcs-acos`: reads `{value}` -> `{"result": acos(value)}` (as f64).
#[allow(dead_code)]
async fn spawn_acos() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let value = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": value.acos() }))
        }),
    );
    serve(app).await
}

/// `srvcs-magnitude`: reads `{vector: [..]}` -> `{"result": ||vector||}` (f64).
#[allow(dead_code)]
async fn spawn_magnitude() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let sum_sq: f64 = body
                .get("vector")
                .and_then(Value::as_array)
                .map(|xs| {
                    xs.iter()
                        .filter_map(Value::as_f64)
                        .map(|x| x * x)
                        .sum::<f64>()
                })
                .unwrap_or(0.0);
            Json(json!({ "result": sum_sq.sqrt() }))
        }),
    );
    serve(app).await
}

/// `srvcs-dotproduct`: reads `{a: [..], b: [..]}` -> `{"result": a·b}` (f64).
#[allow(dead_code)]
async fn spawn_dotproduct() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_array).cloned();
            let b = body.get("b").and_then(Value::as_array).cloned();
            let dot: f64 = match (a, b) {
                (Some(a), Some(b)) => a
                    .iter()
                    .zip(b.iter())
                    .filter_map(|(x, y)| Some(x.as_f64()? * y.as_f64()?))
                    .sum(),
                _ => 0.0,
            };
            Json(json!({ "result": dot }))
        }),
    );
    serve(app).await
}

/// `srvcs-vectorsubtract`: reads `{a: [..], b: [..]}` -> `{"result": [a-b]}`.
#[allow(dead_code)]
async fn spawn_vectorsubtract() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_array).cloned();
            let b = body.get("b").and_then(Value::as_array).cloned();
            let diff: Vec<f64> = match (a, b) {
                (Some(a), Some(b)) => a
                    .iter()
                    .zip(b.iter())
                    .filter_map(|(x, y)| Some(x.as_f64()? - y.as_f64()?))
                    .collect(),
                _ => Vec::new(),
            };
            Json(json!({ "result": diff }))
        }),
    );
    serve(app).await
}

/// Spawn a mock returning a fixed status + body (used for error-path tests).
async fn spawn_fixed(status: StatusCode, body: Value) -> String {
    let app = AxumRouter::new().route(
        "/",
        post(move || {
            let body = body.clone();
            async move { (status, Json(body)) }
        }),
    );
    serve(app).await
}

async fn serve(app: AxumRouter) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn app(deps: Deps) -> axum::Router {
    router(telemetry::metrics_handle_for_tests(), deps)
}

async fn real_deps() -> Deps {
    Deps {
        floatadd_url: spawn_floatadd().await,
    }
}

fn dead_deps() -> Deps {
    Deps {
        floatadd_url: DEAD_URL.to_string(),
    }
}

async fn vectoradd(deps: Deps, a: Value, b: Value) -> (StatusCode, Value) {
    let res = app(deps)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "a": a, "b": b }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn status_of(uri: &str) -> StatusCode {
    app(dead_deps())
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

/// Assert two `f64` lists are equal element-wise within 1e-9.
fn assert_list_close(body: &Value, expected: &[f64]) {
    let got = body["result"].as_array().expect("result is a JSON array");
    assert_eq!(got.len(), expected.len(), "result length mismatch");
    for (g, e) in got.iter().zip(expected.iter()) {
        let g = g.as_f64().expect("element is a JSON number");
        assert!((g - e).abs() < 1e-9, "element {g} not within 1e-9 of {e}");
    }
}

// --- Standard endpoints. ---

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app(dead_deps())
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}

#[tokio::test]
async fn index_reports_identity() {
    let res = app(dead_deps())
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["service"], "srvcs-vectoradd");
    assert_eq!(body["concern"], "vectors: component-wise addition");
    assert_eq!(body["depends_on"], json!(["srvcs-floatadd"]));
}

// --- Correctness cases, against the computing floatadd mock. ---

#[tokio::test]
async fn vectoradd_1_2_3_4_is_4_6() {
    let (status, body) = vectoradd(real_deps().await, json!([1, 2]), json!([3, 4])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["a"], json!([1, 2]));
    assert_eq!(body["b"], json!([3, 4]));
    assert_list_close(&body, &[4.0, 6.0]);
}

#[tokio::test]
async fn vectoradd_empty_is_empty() {
    let (status, body) = vectoradd(real_deps().await, json!([]), json!([])).await;
    assert_eq!(status, StatusCode::OK);
    assert_list_close(&body, &[]);
}

#[tokio::test]
async fn vectoradd_fractional_and_negative() {
    let (status, body) = vectoradd(
        real_deps().await,
        json!([1.5, -2.0, 0.0]),
        json!([0.25, 2.0, -3.5]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_list_close(&body, &[1.75, 0.0, -3.5]);
}

#[tokio::test]
async fn vectoradd_single_element() {
    let (status, body) = vectoradd(real_deps().await, json!([10]), json!([32])).await;
    assert_eq!(status, StatusCode::OK);
    assert_list_close(&body, &[42.0]);
}

#[tokio::test]
async fn echoes_input_vectors() {
    let (status, body) = vectoradd(real_deps().await, json!([1, 2]), json!([3, 4])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["a"], json!([1, 2]));
    assert_eq!(body["b"], json!([3, 4]));
}

// --- Error / edge cases. ---

#[tokio::test]
async fn length_mismatch_is_422() {
    let (status, body) = vectoradd(real_deps().await, json!([1, 2, 3]), json!([4, 5])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "vectors must have equal length");
}

#[tokio::test]
async fn degraded_when_floatadd_dead() {
    let (status, body) = vectoradd(dead_deps(), json!([1, 2]), json!([3, 4])).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-floatadd");
}

#[tokio::test]
async fn forwards_422_from_floatadd() {
    let deps = Deps {
        floatadd_url: spawn_fixed(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "error": "value is not a number" }),
        )
        .await,
    };
    let (status, body) = vectoradd(deps, json!(["nope", 2]), json!([3, 4])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "value is not a number");
}

#[tokio::test]
async fn malformed_floatadd_result_is_500() {
    let deps = Deps {
        floatadd_url: spawn_fixed(StatusCode::OK, json!({ "result": "not-a-number" })).await,
    };
    let (status, body) = vectoradd(deps, json!([1, 2]), json!([3, 4])).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["dependency"], "srvcs-floatadd");
}

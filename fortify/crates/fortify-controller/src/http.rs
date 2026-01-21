use crate::service::{ServiceManager, ServiceSnapshot, ServiceStatus, ServiceType};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde::Serialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn spawn_http_server(
    bind_addr: SocketAddr,
    services: Arc<Mutex<ServiceManager>>,
) -> anyhow::Result<()> {
    let make_svc = make_service_fn(move |_conn| {
        let services = Arc::clone(&services);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let services = Arc::clone(&services);
                async move { handle_request(req, services).await }
            }))
        }
    });

    let server = Server::bind(&bind_addr).serve(make_svc);
    tracing::info!("Controller admin API listening on {}", bind_addr);

    tokio::spawn(async move {
        if let Err(err) = server.await {
            tracing::error!("Controller HTTP server error: {}", err);
        }
    });

    Ok(())
}

async fn handle_request(
    req: Request<Body>,
    services: Arc<Mutex<ServiceManager>>,
) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let snapshots = services.lock().await.snapshots();

    let response = match (method, path.as_str()) {
        (Method::GET, "/services") => json_response(&snapshots),
        (Method::GET, "/nodes") => {
            let nodes: Vec<_> = snapshots
                .iter()
                .filter(|svc| svc.service_type == ServiceType::Node)
                .cloned()
                .collect();
            json_response(&serde_json::json!({ "nodes": nodes, "count": nodes.len() }))
        }
        (Method::GET, "/health") => {
            let running = snapshots
                .iter()
                .filter(|svc| svc.status == ServiceStatus::Running)
                .count();
            let failed = snapshots
                .iter()
                .filter(|svc| svc.status == ServiceStatus::Failed)
                .count();
            json_response(&serde_json::json!({
                "services": snapshots.len(),
                "running": running,
                "failed": failed,
            }))
        }
        (Method::GET, "/") => html_overview(&snapshots),
        _ => not_found(),
    };

    Ok(response)
}

fn json_response<T: Serialize>(value: &T) -> Response<Body> {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".into());
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

fn html_overview(services: &[ServiceSnapshot]) -> Response<Body> {
    let rows = services
        .iter()
        .map(|svc| {
            format!(
                "<tr><td>{}</td><td>{:?}</td><td>{:?}</td><td>{}</td><td>{}</td></tr>",
                svc.id,
                svc.service_type,
                svc.status,
                svc.bind_addr.clone().unwrap_or_else(|| "-".into()),
                svc.mode.clone().unwrap_or_else(|| "-".into())
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Fortify Controller</title>
    <style>
        body {{ font-family: monospace; background: #111; color: #0f0; padding: 20px; }}
        table {{ width: 100%; border-collapse: collapse; }}
        th, td {{ border: 1px solid #0f0; padding: 8px; text-align: left; }}
        th {{ background: #222; }}
    </style>
</head>
<body>
    <h1>⚡ Fortify Controller ⚡</h1>
    <table>
        <thead>
            <tr>
                <th>ID</th>
                <th>Type</th>
                <th>Status</th>
                <th>Bind Addr</th>
                <th>Mode</th>
            </tr>
        </thead>
        <tbody>
            {rows}
        </tbody>
    </table>
</body>
</html>"#,
        rows = rows
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain")
        .body(Body::from("not found"))
        .unwrap()
}

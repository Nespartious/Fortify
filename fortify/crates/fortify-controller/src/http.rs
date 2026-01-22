use crate::service::{ServiceManager, ServiceSnapshot, ServiceStatus, ServiceType};
use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::Serialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

type BoxBody = Full<Bytes>;

pub fn spawn_http_server(
    bind_addr: SocketAddr,
    services: Arc<Mutex<ServiceManager>>,
) -> anyhow::Result<()> {
    tracing::info!("Controller admin API listening on {}", bind_addr);

    tokio::spawn(async move {
        let listener = match TcpListener::bind(bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to bind controller HTTP server: {}", e);
                return;
            }
        };

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            let io = TokioIo::new(stream);
            let services = Arc::clone(&services);

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    let services = Arc::clone(&services);
                    async move { handle_request(req, services).await }
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    tracing::debug!("Connection error: {}", err);
                }
            });
        }
    });

    Ok(())
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    services: Arc<Mutex<ServiceManager>>,
) -> Result<Response<BoxBody>, Infallible> {
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

fn json_response<T: Serialize>(value: &T) -> Response<BoxBody> {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".into());
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

fn html_overview(services: &[ServiceSnapshot]) -> Response<BoxBody> {
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
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

fn not_found() -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("not found")))
        .unwrap()
}

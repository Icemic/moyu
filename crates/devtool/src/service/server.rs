//! Layered static file server and dev-server reverse proxy for web mode.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, bail};
use percent_encoding::percent_decode_str;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

use crate::config::{DEV_SERVER_PORT, platform_dir};

static DEV_SERVER_WARNED: AtomicBool = AtomicBool::new(false);

pub struct StaticFileServer {
    #[allow(dead_code)]
    pub port: u16,
    pub url: String,
    shutdown_tx: watch::Sender<bool>,
}

impl StaticFileServer {
    pub fn close(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

impl Drop for StaticFileServer {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn require_web_engine_assets(project_root: &Path, version: &str) -> Result<PathBuf> {
    let web_path = platform_dir(project_root, version, "web-universal")?;
    if !web_path.is_dir() {
        bail!(
            "Web engine assets not found. Run \"moyu-devtool download\" to download web-universal platform."
        );
    }
    let has_entries = fs::read_dir(&web_path)?.next().is_some();
    if !has_entries {
        bail!("Web engine directory is empty. Run \"moyu-devtool download\" to re-download.");
    }
    Ok(web_path)
}

pub async fn start_static_file_server(
    project_root: &Path,
    web_path: &Path,
    requested_port: u16,
) -> Result<StaticFileServer> {
    let listener = bind_with_retry(requested_port, 10).await?;
    let port = listener.local_addr()?.port();
    let url = format!("http://localhost:{port}");

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let project_root = project_root.to_path_buf();
    let web_path = web_path.to_path_buf();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                accept_res = listener.accept() => {
                    let Ok((stream, _)) = accept_res else {
                        break;
                    };
                    let project_root = project_root.clone();
                    let web_path = web_path.clone();
                    tokio::spawn(async move {
                        let _ = handle_connection(stream, &web_path, &project_root).await;
                    });
                }
            }
        }
    });

    Ok(StaticFileServer {
        port,
        url,
        shutdown_tx,
    })
}

async fn bind_with_retry(base_port: u16, attempts: u16) -> Result<TcpListener> {
    let mut port = base_port;
    for attempt in 0..=attempts {
        match TcpListener::bind(("0.0.0.0", port)).await {
            Ok(listener) => return Ok(listener),
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse && attempt < attempts => {
                eprintln!("warn: Port {port} in use, trying {}...", port + 1);
                port = port.saturating_add(1);
            }
            Err(err) => return Err(err.into()),
        }
    }
    unreachable!()
}

async fn handle_connection(
    mut client: TcpStream,
    web_path: &Path,
    project_root: &Path,
) -> Result<()> {
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 4096];

    let header_end = loop {
        let n = client.read(&mut tmp).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
        if buf.len() > 64 * 1024 {
            write_simple_response(&mut client, 431, "431 Request Header Fields Too Large").await?;
            return Ok(());
        }
    };

    let mut headers_storage = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers_storage);
    let parsed = req.parse(&buf[..header_end])?;
    if !parsed.is_complete() {
        write_simple_response(&mut client, 400, "400 Bad Request").await?;
        return Ok(());
    }

    let method = req.method.unwrap_or("GET").to_string();
    let raw_target = req.path.unwrap_or("/").to_string();
    let req_headers: Vec<(String, String)> = req
        .headers
        .iter()
        .map(|h| {
            (
                h.name.to_string(),
                String::from_utf8_lossy(h.value).to_string(),
            )
        })
        .collect();

    let is_upgrade = req_headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("upgrade") && v.eq_ignore_ascii_case("websocket"));

    let pathname = raw_target.split('?').next().unwrap_or("/");
    if is_upgrade {
        if pathname != "/ws" {
            return Ok(());
        }
        let body_remainder = &buf[header_end..];
        proxy_websocket(client, &method, &raw_target, &req_headers, body_remainder).await?;
        return Ok(());
    }

    if let Some(file_path) = resolve_file_path(&raw_target, web_path, project_root) {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_ascii_lowercase()))
            .unwrap_or_default();
        let mime = mime_type(&ext);

        match tokio::fs::read(&file_path).await {
            Ok(data) => {
                let mut head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n",
                    data.len()
                );
                if ext == ".html" {
                    head.push_str("Cross-Origin-Embedder-Policy: require-corp\r\n");
                    head.push_str("Cross-Origin-Opener-Policy: same-origin\r\n");
                }
                head.push_str("\r\n");
                client.write_all(head.as_bytes()).await?;
                if !method.eq_ignore_ascii_case("HEAD") {
                    client.write_all(&data).await?;
                }
            }
            Err(_) => {
                write_simple_response(&mut client, 500, "500 Internal Server Error").await?;
            }
        }
        return Ok(());
    }

    // Read remaining request body if Content-Length is present before proxying.
    let content_length = req_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = client.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }

    proxy_http(client, &method, &raw_target, &req_headers, &body).await?;
    Ok(())
}

async fn proxy_http(
    mut client: TcpStream,
    method: &str,
    target: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<()> {
    let mut upstream = match TcpStream::connect(("127.0.0.1", DEV_SERVER_PORT)).await {
        Ok(s) => s,
        Err(_) => {
            if !DEV_SERVER_WARNED.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "warn: Dev server not reachable on port {DEV_SERVER_PORT}; bundler output will fail to load."
                );
            }
            let msg = format!(
                "502 Bad Gateway: no dev server on port {DEV_SERVER_PORT}. Start it and reload."
            );
            write_simple_response(&mut client, 502, &msg).await?;
            return Ok(());
        }
    };

    let mut req_head = format!("{method} {target} HTTP/1.1\r\n");
    req_head.push_str(&format!("Host: 127.0.0.1:{DEV_SERVER_PORT}\r\n"));
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("host") || k.eq_ignore_ascii_case("connection") {
            continue;
        }
        req_head.push_str(&format!("{k}: {v}\r\n"));
    }
    req_head.push_str("Connection: close\r\n\r\n");

    upstream.write_all(req_head.as_bytes()).await?;
    if !body.is_empty() {
        upstream.write_all(body).await?;
    }
    upstream.flush().await?;

    let _ = tokio::io::copy(&mut upstream, &mut client).await;
    Ok(())
}

async fn proxy_websocket(
    mut client: TcpStream,
    method: &str,
    target: &str,
    headers: &[(String, String)],
    remainder: &[u8],
) -> Result<()> {
    let Ok(mut upstream) = TcpStream::connect(("127.0.0.1", DEV_SERVER_PORT)).await else {
        return Ok(());
    };

    let mut req_head = format!("{method} {target} HTTP/1.1\r\n");
    req_head.push_str(&format!("Host: 127.0.0.1:{DEV_SERVER_PORT}\r\n"));
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("host") {
            continue;
        }
        req_head.push_str(&format!("{k}: {v}\r\n"));
    }
    req_head.push_str("\r\n");

    upstream.write_all(req_head.as_bytes()).await?;
    if !remainder.is_empty() {
        upstream.write_all(remainder).await?;
    }

    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    Ok(())
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

async fn write_simple_response(client: &mut TcpStream, status: u16, body: &str) -> Result<()> {
    let reason = match status {
        400 => "Bad Request",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "Error",
    };
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    client.write_all(resp.as_bytes()).await?;
    Ok(())
}

fn resolve_file_path(url_path: &str, web_path: &Path, project_root: &Path) -> Option<PathBuf> {
    let without_query = url_path.split('?').next().unwrap_or("/");
    let without_hash = without_query.split('#').next().unwrap_or("/");
    let decoded = percent_decode_str(without_hash).decode_utf8().ok()?;

    let mut rel = PathBuf::new();
    for seg in Path::new(decoded.as_ref()).components() {
        match seg {
            Component::Normal(part) => rel.push(part),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            Component::ParentDir => {
                if !rel.pop() {
                    return None;
                }
            }
        }
    }

    for root in [web_path, project_root] {
        let candidate = root.join(&rel);
        if !candidate.exists() {
            continue;
        }
        if candidate.is_file() {
            return Some(candidate);
        }
        if candidate.is_dir() {
            let index = candidate.join("index.html");
            if index.is_file() {
                return Some(index);
            }
        }
    }

    None
}

fn mime_type(ext: &str) -> &'static str {
    match ext {
        ".html" => "text/html; charset=utf-8",
        ".js" | ".mjs" => "application/javascript; charset=utf-8",
        ".css" => "text/css; charset=utf-8",
        ".json" => "application/json; charset=utf-8",
        ".wasm" => "application/wasm",
        ".png" => "image/png",
        ".jpg" | ".jpeg" => "image/jpeg",
        ".gif" => "image/gif",
        ".svg" => "image/svg+xml",
        ".ico" => "image/x-icon",
        ".webp" => "image/webp",
        ".mp3" => "audio/mpeg",
        ".ogg" => "audio/ogg",
        ".opus" => "audio/opus",
        ".wav" => "audio/wav",
        ".mp4" => "video/mp4",
        ".webm" => "video/webm",
        ".woff" => "font/woff",
        ".woff2" => "font/woff2",
        ".ttf" => "font/ttf",
        ".otf" => "font/otf",
        ".txt" => "text/plain; charset=utf-8",
        ".xml" => "application/xml",
        _ => "application/octet-stream",
    }
}

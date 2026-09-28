async fn log_middleware(req: Request<Body>, next: Next) -> Response {
    // WebSocket upgrade 请求必须原样透传，不能读 body
    let is_ws_upgrade = req
        .headers()
        .get("upgrade")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    if is_ws_upgrade {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let query = req.uri().query().map(|s| s.to_string());
        info!("");
        info!("🔌 [WS-UPGRADE] {} {}{}", method, path,
            query.map(|q| format!("?{}", q)).unwrap_or_default());
        info!("   Sec-WebSocket-Key: {}", header_str(req.headers(), "sec-websocket-key").unwrap_or_default());
        info!("   Sec-WebSocket-Version: {}", header_str(req.headers(), "sec-websocket-version").unwrap_or_default());

        let resp = next.run(req).await;

        info!("   → upgrade 响应: {}", resp.status());
        return resp;
    }

    let start = Instant::now();
    let req_id = REQ_COUNTER.fetch_add(1, Ordering::Relaxed);

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|s| s.to_string());
    let version = req.version();
    let is_video = path.starts_with("/Videos/");
    let is_image = path.starts_with("/Images/");

    let ua = header_str(req.headers(), "user-agent");
    let auth = header_str(req.headers(), "authorization");
    let authx = header_str(req.headers(), "authx");
    let referer = header_str(req.headers(), "referer");
    let range = header_str(req.headers(), "range");
    let content_type = header_str(req.headers(), "content-type");

    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("-").trim().to_string())
        .or_else(|| {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.to_string())
        })
        .unwrap_or_else(|| "-".into());

    let (parts, body) = req.into_parts();
    let body_bytes = if is_video || is_image {
        bytes::Bytes::new()
    } else {
        axum::body::to_bytes(body, 8192)
            .await
            .unwrap_or_else(|_| bytes::Bytes::new())
    };
    let body_len = body_bytes.len();
    let body_preview = if body_len > 0 {
        Some(truncate(&String::from_utf8_lossy(&body_bytes), 500))
    } else {
        None
    };

    let req = Request::from_parts(parts, Body::from(body_bytes));

    let video_sample = env_u64("VIDEO_LOG_SAMPLE", 100).max(1);
    let image_sample = env_u64("IMAGE_LOG_SAMPLE", 50).max(1);

    let log_request = if is_video {
        VIDEO_COUNTER.fetch_add(1, Ordering::Relaxed) % video_sample == 0
    } else if is_image {
        IMAGE_COUNTER.fetch_add(1, Ordering::Relaxed) % image_sample == 0
    } else {
        true
    };

    if log_request && !is_image {
        info!("");
        info!("┌─ [req#{}] {} {} {:?}", req_id, method, path, version);
        info!("│  from        : {}", client_ip);
        info!("│  user-agent  : {}", ua.as_deref().unwrap_or("-"));
        if let Some(q) = &query {
            info!("│  query       : {}", q);
        }
        if let Some(ct) = &content_type {
            info!("│  content-type: {}", ct);
        }
        if let Some(a) = &auth {
            info!("│  authorization: {}", truncate(a, 100));
        }
        if let Some(a) = &authx {
            info!("│  authx       : {}", truncate(a, 120));
        }
        if let Some(r) = &referer {
            info!("│  referer     : {}", truncate(r, 80));
        }
        if let Some(b) = &body_preview {
            info!("│  body ({}B)  : {}", body_len, b);
        }
    }

    let resp = next.run(req).await;

    let status = resp.status().as_u16();
    let elapsed = start.elapsed();

    if is_video {
        if log_request {
            info!(
                "🎬 [req#{}] {} {} → {} ({:?}) Range={}",
                req_id, method, path, status, elapsed,
                range.as_deref().unwrap_or("-")
            );
        }
    } else if is_image {
        if log_request {
            info!("🖼️  [req#{}] Images → {} ({:?})", req_id, status, elapsed);
        }
    } else {
        info!("│  → status: {} {} ({:?})", status, reason(status), elapsed);
        info!("└─ [req#{}] done", req_id);
    }

    resp
}
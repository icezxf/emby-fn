use axum::extract::WebSocketUpgrade;
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn};

/// GET /websocket
/// 飞牛客户端登录后会建立长连接，我们只做握手并保持连接
pub async fn handle_websocket(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: axum::extract::ws::WebSocket) {
    info!("🔌 [WS] 客户端建立 WebSocket 连接");
    let (mut sender, mut receiver) = socket.split();

    // 客户端期望服务端主动推一条初始消息，随便发一个合法的 JSON 保持连接
    let hello = serde_json::json!({
        "req": "sys.websocket.connected",
        "reqid": 0,
        "code": 0,
        "msg": "",
        "data": {}
    });
    if sender
        .send(axum::extract::ws::Message::Text(hello.to_string().into()))
        .await
        .is_err()
    {
        warn!("🔌 [WS] 发送初始消息失败");
        return;
    }

    let mut count = 0u64;
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(m) => {
                count += 1;
                let text = match &m {
                    axum::extract::ws::Message::Text(t) => t.to_string(),
                    axum::extract::ws::Message::Binary(b) => format!("<binary {}B>", b.len()),
                    axum::extract::ws::Message::Ping(_) => "<ping>".to_string(),
                    axum::extract::ws::Message::Pong(_) => "<pong>".to_string(),
                    axum::extract::ws::Message::Close(_) => "<close>".to_string(),
                };
                info!("🔌 [WS] 收到消息 #{}: {}", count, truncate(&text, 300));

                // 尝试解析消息里的 req/reqid，回一个空响应
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    let req = json.get("req").and_then(|v| v.as_str()).unwrap_or("");
                    let reqid = json.get("reqid").cloned().unwrap_or(serde_json::json!(0));
                    if !req.is_empty() {
                        let reply = serde_json::json!({
                            "req": req,
                            "reqid": reqid,
                            "code": 0,
                            "msg": "",
                            "data": {}
                        });
                        if sender
                            .send(axum::extract::ws::Message::Text(reply.to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        info!("🔌 [WS] 已回复 req={} reqid={}", req, reqid);
                    }
                }
            }
            Err(e) => {
                warn!("🔌 [WS] 连接错误: {}", e);
                break;
            }
        }
    }
    info!("🔌 [WS] 连接关闭 (共收到 {} 条消息)", count);
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n).collect();
        format!("{}...", t)
    }
}
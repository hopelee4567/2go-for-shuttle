use std::net::SocketAddr;
use std::process::Command;
use shuttle_runtime::{SecretStore, tracing};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;

// 主处理函数，返回 Axum 路由
#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    // --- 1. 从密钥中获取所有配置 ---
    let uuid = secrets.get("UUID").expect("UUID must be set");
    let argo_domain = secrets.get("ARGO_DOMAIN").expect("ARGO_DOMAIN must be set");
    let argo_auth = secrets.get("ARGO_AUTH").expect("ARGO_AUTH (Tunnel Token) must be set");
    let sub_path = secrets.get("SUB_PATH").unwrap_or_else(|| "sub".to_string());

    // --- 2. 启动 Xray 核心 ---
    // Xray 配置现在直接在这里用代码生成，更清晰、更不容易出错
    let xray_config = format!(r#"
    {{
        "log": {{ "loglevel": "none" }},
        "inbounds": [
            {{
                "port": 8000,
                "listen": "127.0.0.1",
                "protocol": "vless",
                "settings": {{ "clients": [{{ "id": "{}" }}], "decryption": "none" }},
                "streamSettings": {{ "network": "ws", "security": "none", "wsSettings": {{ "path": "/proxy" }} }}
            }}
        ],
        "outbounds": [{{ "protocol": "freedom" }}]
    }}
    "#, uuid);
    
    // 将配置写入文件
    std::fs::write("/tmp/config.json", xray_config).expect("Unable to write xray config file");

    // 启动 Xray
    Command::new("/usr/bin/xray")
        .args(["run", "-c", "/tmp/config.json"])
        .spawn()
        .expect("failed to start xray");
    tracing::info!("Xray core started");

    // --- 3. 启动 Cloudflare Tunnel ---
    Command::new("/usr/bin/cloudflared")
        .args([
            "tunnel",
            "--no-autoupdate",
            "run",
            "--token",
            &argo_auth,
        ])
        .spawn()
        .expect("failed to start cloudflared");
    tracing::info!("Cloudflare tunnel started");

    // --- 4. 设置网页路由，用于提供订阅链接 ---
    let sub_content = generate_subscription_content(&uuid, &argo_domain);
    let router = Router::new().route(
        &format!("/{}", sub_path),
        get(move || async { sub_content }),
    );

    Ok(router.into())
}

// 辅助函数：生成订阅内容
fn generate_subscription_content(uuid: &str, domain: &str) -> String {
    let vless_link = format!("vless://{}@{}:443?encryption=none&security=tls&sni={}&type=ws&host={}&path=%2Fproxy#Shuttle-CF", uuid, domain, domain, domain);
    BASE64_STANDARD.encode(vless_link)
}

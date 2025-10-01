use axum::{routing::get, Router};
use shuttle_runtime::{tracing, SecretStore}; // 修正 1：正确引入 tracing
use std::net::SocketAddr;                     // 修正 2：正确引入 SocketAddr
use std::process::Command;
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
    let xray_port = "8000"; // Xray 监听的内部端口
    let web_port = "8080";  // 网页服务监听的内部端口

    // --- 2. 启动 Xray 核心 ---
    let xray_config = format!(r#"
    {{
        "log": {{ "loglevel": "none" }},
        "inbounds": [
            {{
                "port": {},
                "listen": "127.0.0.1",
                "protocol": "vless",
                "settings": {{ "clients": [{{ "id": "{}" }}], "decryption": "none" }},
                "streamSettings": {{ "network": "ws", "security": "none", "wsSettings": {{ "path": "/proxy" }} }},
                "fallbacks": [
                    {{ "path": "/{}", "dest": {} }}
                ]
            }}
        ],
        "outbounds": [{{ "protocol": "freedom" }}]
    }}
    "#, xray_port, uuid, sub_path, web_port);
    
    std::fs::write("/tmp/config.json", xray_config).expect("Unable to write xray config file");

    Command::new("/usr/bin/xray")
        .args(["run", "-c", "/tmp/config.json"])
        .spawn()
        .expect("failed to start xray");
    tracing::info!("Xray core started on port {}", xray_port); // 修正 3：修正日志调用

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
    tracing::info!("Cloudflare tunnel started, pointing to Xray on port {}", xray_port); // 修正 3：修正日志调用

    // --- 4. 设置网页路由，用于提供订阅链接 ---
    let sub_content = generate_subscription_content(&uuid, &argo_domain);
    let router = Router::new().route(
        &format!("/{}", sub_path),
        get(move || async { sub_content.clone() }),
    );
    
    // 将网页服务绑定到指定的内部端口
    let addr: SocketAddr = format!("0.0.0.0:{}", web_port).parse().unwrap();
    tracing::info!("Subscription web service listening on {}", addr); // 修正 3：修正日志调用
    
    Ok(router.into())
}

// 辅助函数：生成订阅内容
fn generate_subscription_content(uuid: &str, domain: &str) -> String {
    let vless_link = format!("vless://{}@{}:443?encryption=none&security=tls&sni={}&type=ws&host={}&path=%2Fproxy#Shuttle-CF", uuid, domain, domain, domain);
    BASE64_STANDARD.encode(vless_link)
}

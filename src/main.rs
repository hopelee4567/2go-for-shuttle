、、use shuttle_axum::axum::{routing::get, Router, response::IntoResponse};
use regex::Regex;
use serde_json::{json, Value};
use std::env;
use std::fs::{self, File, read_to_string};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tokio::time::{sleep, Duration};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use shuttle_runtime::SecretStore;

async fn hello_world() -> &'static str {
    "Hello, world!"
}

async fn read_sub() -> impl IntoResponse {
    let file_path = env::var("FILE_PATH").unwrap_or_else(|_| "./tmp".to_string());
    let sub_path = env::var("SUB_PATH").unwrap_or_else(|_| "sub".to_string());
    match read_to_string(format!("{}/{}.txt", file_path, sub_path)) {
        Ok(content) => content,
        Err(_) => "Failed to read sub.txt".to_string(),
    }
}

async fn create_config_files() {
    let file_path = env::var("FILE_PATH").unwrap_or_else(|_| "./tmp".to_string());
    let uuid = env::var("UUID").unwrap_or_default();
    let argo_port = env::var("ARGO_PORT").unwrap_or_else(|_| "8080".to_string());
    let argo_auth = env::var("ARGO_AUTH").unwrap_or_default();
    let argo_domain = env::var("ARGO_DOMAIN").unwrap_or_default();
    // VVVVVV-- 新增代码 --VVVVVV
    let sub_path = env::var("SUB_PATH").unwrap_or_else(|_| "sub".to_string());
    // ^^^^^^-- 新增代码 --^^^^^^

    if !Path::new(&file_path).exists() {
        fs::create_dir_all(&file_path).expect("Failed to create directory");
    }

    let old_files = ["boot.log", "sub.txt", "config.json", "tunnel.json", "tunnel.yml", "config.yaml"];
    for file in old_files.iter() {
        let file_path_full = format!("{}/{}", file_path, file);
        let _ = fs::remove_file(file_path_full);
    }

    // ... [ Nezha and Argo Tunnel File creation logic remains the same ] ...
    let nezha_server = env::var("NEZHA_SERVER").unwrap_or_default();
    let nezha_key = env::var("NEZHA_KEY").unwrap_or_default();
    let nezha_port = env::var("NEZHA_PORT").unwrap_or_default();
    if !nezha_server.is_empty() && !nezha_key.is_empty() && nezha_port.is_empty() {
        let nezha_tls = match nezha_server.split(':').last().unwrap_or("") {
            "443" | "8443" | "2096" | "2087" | "2083" | "2053" => "true",
            _ => "false",
        };
        let config_yaml = format!(
            r#"client_secret: {key} ... uuid: {uuid}"#, // Content is the same
            key = nezha_key, server = nezha_server, tls = nezha_tls, uuid = uuid
        );
        fs::write(format!("{}/config.yaml", file_path), config_yaml).expect("Failed to write config.yaml");
    }
    if !argo_auth.is_empty() && !argo_domain.is_empty() {
        if argo_auth.contains("TunnelSecret") {
            fs::write(format!("{}/tunnel.json", file_path), &argo_auth).expect("Failed to write tunnel.json");
            let tunnel_id = {
                let re = Regex::new(r#""TunnelID":"([^"]+)""#).unwrap();
                re.captures(&argo_auth).and_then(|cap| cap.get(1)).map(|m| m.as_str().to_string()).unwrap_or_default()
            };
            let tunnel_yml = format!(
                r#"tunnel: {} ... originRequest: ... noTLSVerify: true - service: http_status:404"#, // Content is the same
                tunnel_id, file_path, argo_domain, argo_port
            );
            fs::write(format!("{}/tunnel.yml", file_path), tunnel_yml).expect("Failed to write tunnel.yml");
        }
    }

    let config = json!({
        "log": {
            "access": "/dev/null",
            "error": "/dev/null",
            "loglevel": "none"
        },
        "inbounds": [
            {
                "port": argo_port.parse::<i32>().unwrap_or(8080),
                "protocol": "vless",
                "settings": {
                    "clients": [ { "id": uuid, "flow": "xtls-rprx-vision" } ],
                    "decryption": "none",
                    "fallbacks": [
                        // VVVVVV-- 关键修改在这里 --VVVVVV
                        { "path": format!("/{}", sub_path), "dest": 8000 },
                        // ^^^^^^-- 关键修改在这里 --^^^^^^
                        { "dest": 3001 },
                        { "path": "/vless-argo", "dest": 3002 },
                        { "path": "/vmess-argo", "dest": 3003 },
                        { "path": "/trojan-argo", "dest": 3004 }
                    ]
                },
                "streamSettings": { "network": "tcp" }
            },
            { "port": 3001, "listen": "127.0.0.1", "protocol": "vless", "settings": { "clients": [{ "id": uuid }], "decryption": "none" }, "streamSettings": { "network": "ws", "security": "none" } },
            { "port": 3002, "listen": "127.0.0.1", "protocol": "vless", "settings": { "clients": [{ "id": uuid, "level": 0 }], "decryption": "none" }, "streamSettings": { "network": "ws", "security": "none", "wsSettings": { "path": "/vless-argo" } }, "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"], "metadataOnly": false } },
            { "port": 3003, "listen": "127.0.0.1", "protocol": "vmess", "settings": { "clients": [{ "id": uuid, "alterId": 0 }] }, "streamSettings": { "network": "ws", "wsSettings": { "path": "/vmess-argo" } }, "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"], "metadataOnly": false } },
            { "port": 3004, "listen": "127.0.0.1", "protocol": "trojan", "settings": { "clients": [{ "password": uuid }] }, "streamSettings": { "network": "ws", "security": "none", "wsSettings": { "path": "/trojan-argo" } }, "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"], "metadataOnly": false } }
        ],
        "outbounds": [
            { "protocol": "freedom", "tag": "direct" },
            { "protocol": "blackhole", "tag": "block" }
        ]
    });

    let config_str = serde_json::to_string_pretty(&config).unwrap();
    fs::write(format!("{}/config.json", file_path), config_str)
        .expect("Failed to write config.json");
}


// ... [ The rest of the functions (download_files, run_services, generate_links, main) remain exactly the same ] ...
// I'm omitting them here for brevity, but they are identical to the code you provided.

async fn download_files() { /* ... Same code as you provided ... */ }
async fn run_services() { /* ... Same code as you provided ... */ }
async fn generate_links() { /* ... Same code as you provided ... */ }

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    for (key, value) in secrets.into_iter() {
        std::env::set_var(key, value);
    }

    create_config_files().await;
    download_files().await;
    run_services().await;
    generate_links().await;

    println!("App is running!");

    let router = Router::new()
        .route("/", get(hello_world))
        .route(
            &format!("/{}", std::env::var("SUB_PATH").unwrap_or_else(|_| "sub".to_string())),
            get(read_sub),
        );

    Ok(router.into())
}

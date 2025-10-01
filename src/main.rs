use shuttle_axum::axum::{routing::get, Router, response::IntoResponse};
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

// The functions hello_world, read_sub, create_config_files, download_files, and generate_links remain unchanged.
// I am omitting them for brevity. Ensure they are present in your final file.
async fn hello_world() -> &'static str { "Hello, world!" }
async fn read_sub(secrets: &SecretStore) -> impl IntoResponse {
    let file_path = secrets.get("FILE_PATH").unwrap_or_else(|| "./tmp".to_string());
    let sub_path = secrets.get("SUB_PATH").unwrap_or_else(|| "sub".to_string());
    match read_to_string(format!("{}/{}.txt", file_path, sub_path)) {
        Ok(content) => content,
        Err(_) => "Failed to read sub.txt".to_string(),
    }
}
async fn create_config_files(secrets: &SecretStore) {
    // This function's content is the same as the previous correct version.
    let file_path = secrets.get("FILE_PATH").unwrap_or_else(|| "./tmp".to_string());
    let uuid = secrets.get("UUID").unwrap_or_default();
    let argo_port = secrets.get("ARGO_PORT").unwrap_or_else(|| "8080".to_string());
    let sub_path = secrets.get("SUB_PATH").unwrap_or_else(|| "sub".to_string());
    if !Path::new(&file_path).exists() { fs::create_dir_all(&file_path).expect("Failed to create directory"); }
    let old_files = ["boot.log", "sub.txt", "config.json", "tunnel.json", "tunnel.yml", "config.yaml"];
    for file in old_files.iter() { let _ = fs::remove_file(format!("{}/{}", file_path, file)); }
    let config = json!({
        "log": { "access": "/dev/null", "error": "/dev/null", "loglevel": "none" },
        "inbounds": [
            {
                "port": argo_port.parse::<i32>().unwrap_or(8080), "protocol": "vless",
                "settings": {
                    "clients": [ { "id": uuid, "flow": "xtls-rprx-vision" } ], "decryption": "none",
                    "fallbacks": [
                        { "path": format!("/{}", sub_path), "dest": 8000 },
                        { "dest": 3001 },
                        { "path": "/vless-argo", "dest": 3002 },
                        { "path": "/vmess-argo", "dest": 3003 },
                        { "path": "/trojan-argo", "dest": 3004 }
                    ]
                },
                "streamSettings": { "network": "tcp" }
            },
            { "port": 3001, "listen": "127.0.0.1", "protocol": "vless", "settings": { "clients": [{ "id": uuid.clone() }], "decryption": "none" }, "streamSettings": { "network": "ws", "security": "none" } },
            { "port": 3002, "listen": "127.0.0.1", "protocol": "vless", "settings": { "clients": [{ "id": uuid.clone(), "level": 0 }], "decryption": "none" }, "streamSettings": { "network": "ws", "security": "none", "wsSettings": { "path": "/vless-argo" } }, "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"], "metadataOnly": false } },
            { "port": 3003, "listen": "127.0.0.1", "protocol": "vmess", "settings": { "clients": [{ "id": uuid.clone(), "alterId": 0 }] }, "streamSettings": { "network": "ws", "wsSettings": { "path": "/vmess-argo" } }, "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"], "metadataOnly": false } },
            { "port": 3004, "listen": "127.0.0.1", "protocol": "trojan", "settings": { "clients": [{ "password": uuid.clone() }] }, "streamSettings": { "network": "ws", "security": "none", "wsSettings": { "path": "/trojan-argo" } }, "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"], "metadataOnly": false } }
        ],
        "outbounds": [ { "protocol": "freedom", "tag": "direct" }, { "protocol": "blackhole", "tag": "block" } ]
    });
    fs::write(format!("{}/config.json", file_path), serde_json::to_string_pretty(&config).unwrap()).expect("Failed to write config.json");
}
async fn download_files(secrets: &SecretStore) {
    // This function's content is the same as the previous correct version.
    let file_path = secrets.get("FILE_PATH").unwrap_or_else(|| "./tmp".to_string());
    let arch = Command::new("uname").arg("-m").output().map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string()).unwrap_or_default();
    let file_info = match arch.as_str() {
        "arm" | "arm64" | "aarch64" => vec![("https://arm64.ssss.nyc.mn/2go", "bot"), ("https://arm64.ssss.nyc.mn/web", "web")],
        "amd64" | "x86_64" | "x86" => vec![("https://amd64.ssss.nyc.mn/2go", "bot"), ("https://amd64.ssss.nyc.mn/web", "web")],
        _ => vec![],
    };
    for (url, filename) in file_info {
        let filepath = format!("{}/{}", file_path, filename);
        if !Path::new(&filepath).exists() {
            Command::new("curl").args(["-L", "-sS", "-o", &filepath, url]).status().expect("Failed to download file");
            Command::new("chmod").args(["777", &filepath]).status().expect("Failed to set permissions");
        }
    }
}
async fn generate_links(secrets: &SecretStore) {
    // This function's content is the same as the previous correct version.
    let file_path = secrets.get("FILE_PATH").unwrap_or_else(|| "./tmp".to_string());
    sleep(Duration::from_secs(6)).await;
    let argo_domain = secrets.get("ARGO_DOMAIN").unwrap_or_default();
    println!("ArgoDomain: {}", argo_domain);
    sleep(Duration::from_secs(2)).await;
    let isp = Command::new("curl").args(["-s", "https://speed.cloudflare.com/meta"]).output().ok().and_then(|output| {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let v: Value = serde_json::from_str(&output_str).unwrap_or(json!({}));
        Some(format!("{}-{}", v["country"].as_str().unwrap_or(""), v["asOrganization"].as_str().unwrap_or("")).replace(" ", "_"))
    }).unwrap_or_default();
    sleep(Duration::from_secs(2)).await;
    let uuid = secrets.get("UUID").unwrap_or_default();
    let cfip = secrets.get("CFIP").unwrap_or_default();
    let cfport = secrets.get("CFPORT").unwrap_or_default();
    let name = secrets.get("NAME").unwrap_or_default();
    let vmess_config = json!({ "v": "2", "ps": format!("{}-{}", name, isp), "add": cfip, "port": cfport, "id": uuid, "aid": "0", "scy": "none", "net": "ws", "type": "none", "host": argo_domain, "path": "/vmess-argo?ed=2560", "tls": "tls", "sni": argo_domain, "alpn": "", "fp": "chrome" });
    let mut list_file = File::create(format!("{}/list.txt", file_path)).expect("Failed to create list.txt");
    writeln!(list_file, "vless://{}@{}:{}?encryption=none&security=tls&sni={}&fp=chrome&type=ws&host={}&path=%2Fvless-argo%3Fed%3D2560#{}-{}", uuid, cfip, cfport, argo_domain, argo_domain, name, isp).unwrap();
    writeln!(list_file, "\nvmess://{}", BASE64_STANDARD.encode(serde_json::to_string(&vmess_config).unwrap())).unwrap();
    writeln!(list_file, "\ntrojan://{}@{}:{}?security=tls&sni={}&fp=chrome&type=ws&host={}&path=%2Ftrojan-argo%3Fed%3D2560#{}-{}", uuid, cfip, cfport, argo_domain, argo_domain, name, isp).unwrap();
    let list_content = fs::read_to_string(format!("{}/list.txt", file_path)).expect("Failed to read list.txt");
    let sub_content = BASE64_STANDARD.encode(list_content.as_bytes());
    fs::write(format!("{}/sub.txt", file_path), &sub_content).expect("Failed to write sub.txt");
    println!("\n{}", sub_content);
    let _ = fs::remove_file(format!("{}/list.txt", file_path));
}


// --- This is the only function with a change ---
async fn run_services(secrets: &SecretStore) {
    let file_path = secrets.get("FILE_PATH").unwrap_or_else(|| "./tmp".to_string());
    
    if Path::new(&format!("{}/web", file_path)).exists() {
        Command::new(format!("{}/web", file_path))
            .args(["-c", &format!("{}/config.json", file_path)])
            .spawn()
            .expect("Failed to start web");
    }

    sleep(Duration::from_secs(2)).await;

    if Path::new(&format!("{}/bot", file_path)).exists() {
        let argo_auth = secrets.get("ARGO_AUTH").unwrap_or_default();
        let tunnel_id = secrets.get("TUNNEL_ID").unwrap_or_default();
        
        // Add the "--no-autoupdate" flag to prevent the crash
        let args = vec![
            "tunnel",
            "--no-autoupdate", // This is the crucial fix
            "run",
            "--token",
            &argo_auth,
            &tunnel_id
        ];

        Command::new(format!("{}/bot", file_path))
            .args(&args)
            .spawn()
            .expect("Failed to start bot with token and ID");
    }
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    create_config_files(&secrets).await;
    download_files(&secrets).await;
    run_services(&secrets).await;
    generate_links(&secrets).await;

    println!("App is running!");

    let sub_path = secrets.get("SUB_PATH").unwrap_or_else(|| "sub".to_string());
    let router = Router::new()
        .route("/", get(hello_world))
        .route(
            &format!("/{}", sub_path),
            get(move || async move { read_sub(&secrets).await }),
        );

    Ok(router.into())
}

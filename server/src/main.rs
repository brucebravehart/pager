use axum::http::response;
use axum::ServiceExt;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig; // Import TLS config
use base64ct::{Base64UrlUnpadded, Encoding};
use dotenvy::dotenv;
use rustls::ServerConfig;
use rustls_acme::{caches::DirCache, futures_rustls::rustls, AcmeConfig};
use serde::{de::value, Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::normalize_path::NormalizePathLayer;
// use web_push::*;
use web_push_native::{
    jwt_simple::algorithms::ES256KeyPair, p256::PublicKey, Auth, Error, WebPushBuilder,
};

// Matches your JavaScript payload
#[derive(Serialize, Deserialize, Default)]
struct Db {
    usernames: Vec<String>,
    sub_objs: Vec<Value>,
}

const DB_PATH: &str = "data.json";

#[derive(Serialize)]
struct ApiResponse {
    message: String,
}

#[tokio::main]
async fn main() {
    // get env vars
    dotenv().ok();

    // get Port from render, else is 10000
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    // Initialize DB file if it doesn't exist
    if fs::metadata(DB_PATH).await.is_err() {
        let initial_db = serde_json::to_string(&Db::default()).unwrap();
        fs::write(DB_PATH, initial_db)
            .await
            .expect("Failed to create DB file");
    }

    // network stuff

    let cors = CorsLayer::new()
        .allow_origin(
            "https://brucebravehart.github.io"
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let router = Router::new()
        .route("/users", get(get_users))
        .route("/register_user", post(register_user))
        .route("/send-push", post(send_push));
    // Add CORS so your frontend can actually talk to it

    let app = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(cors)
        .service(router);

    // Load your SSL Certificates
    // You need 'cert.pem' and 'key.pem' in your project folder
    /*let config = RustlsConfig::from_pem_file("cert.pem", "key.pem")
    .await
    .expect("Failed to load certificates. Do they exist?");*/

    /*
    let config = AcmeConfig::new(["pager-87gw.onrender.com"]) // Replace with your domain
        .contact(["mailto:asdf@gmail.com"]) // Replace with your email
        .cache(DirCache::new(PathBuf::from("./rustls_acme_cache")))
        .directory_lets_encrypt(true); // Use production Let's Encrypt

    let mut state = config.state();
    let resolver = state.resolver();

    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(resolver);

    let rustls_config = Arc::new(server_config);
    let acceptor = state.axum_acceptor(rustls_config);*/

    /*
    tokio::spawn(async move {
        loop {
            match state.next().await {
                Some(Ok(event)) => println!("ACME Event: {:?}", event),
                Some(Err(e)) => eprintln!("ACME Error: {:?}", e),
                None => break,
            }
        }
    });*/

    // Binding to port 443 requires sudo on Linux
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, tower::make::Shared::new(app))
        .await
        .unwrap(); // old axum setup without tls*/

    /*axum_server::bind(addr)
    .acceptor(acceptor)
    .serve(tower::make::Shared::new(app))
    .await
    .unwrap();*/
}

// GET /users
async fn get_users() -> Json<Vec<String>> {
    let db = read_db().await;
    Json(db.usernames)
}

// POST /register_user
async fn register_user(Json(payload): Json<Value>) -> impl axum::response::IntoResponse {
    println!("received request");
    let mut db = read_db().await;
    if db.usernames.len() < 20 {
        let name = payload["name"].as_str().unwrap_or("Unknown").to_string();
        let sub_obj = payload["subObj"].clone();
        println!("{}", name);
        db.usernames.push(name);
        db.sub_objs.push(sub_obj);
        write_db(db).await;

        let response = ApiResponse {
            message: "User registered".to_string(),
        };
        (StatusCode::OK, Json(response))
    } else {
        let response = ApiResponse {
            message: "User limit reached".to_string(),
        };
        (StatusCode::OK, Json(response))
    }
}

// POST /send-push
async fn send_push(Json(payload): Json<Value>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let vapid_public_key =
        "BDspVj_KfBb-AOxX8zg69l74H_YRwHXr_D6mk0gdqxKy0UOqFRn1wJeD5JIvgGiSvtbq9feY0J0O4ytzaUzWxJU";
    let vapid_private_key = env::var("VAPID_PRIVATE_KEY").expect("VAPID_PRIVATE_KEY must be set");

    let db = read_db().await;
    let usernames = &db.usernames;
    let sub_objs = &db.sub_objs;

    let trigger_user = payload["name"].as_str().unwrap_or("Unknown").to_string();
    let trigger_sub_obj = payload["subObj"].clone();

    let index = sub_objs
        .iter()
        .position(|json| json["endpoint"].as_str() == trigger_sub_obj["endpoint"].as_str());

    // Loop through and send to everyone else
    if let Some(skip_idx) = index {
        println!(
            "Sender identified as index {}. Broadcasting to others...",
            skip_idx
        );

        let decoded_bytes = Base64UrlUnpadded::decode_vec(&vapid_private_key)
            .expect("VAPID private key is not valid base64");
        let key_pair = ES256KeyPair::from_bytes(&decoded_bytes)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for (i, sub) in db.sub_objs.iter().enumerate() {
            if i == skip_idx {
                continue; // Skip the person who triggered the push
            }

            // This is where you call the actual push logic for each 'sub'
            println!("Sending notification to user: {}", db.usernames[i]);

            // decode sub_obj
            let crnt_sub_obj = sub.clone();

            let endpoint: &str = crnt_sub_obj
                .get("endpoint")
                .and_then(|v| v.as_str())
                .unwrap();
            let p256dh: &str = crnt_sub_obj.get("p256dh").and_then(|v| v.as_str()).unwrap();
            let auth: &str = crnt_sub_obj.get("auth").and_then(|v| v.as_str()).unwrap();

            let endpoint = endpoint
                .parse()
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid endpoint: {}", e)))?;
            let p256dh = Base64UrlUnpadded::decode_vec(p256dh).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid p256dh base64: {}", e),
                )
            })?;
            let public_key = PublicKey::from_sec1_bytes(&p256dh).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid public key bytes: {}", e),
                )
            })?;
            let auth_bytes = Base64UrlUnpadded::decode_vec(auth).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid auth base64: {}", e),
                )
            })?;

            // 3. Construct the message
            let builder =
                WebPushBuilder::new(endpoint, public_key, Auth::clone_from_slice(&auth_bytes))
                    .with_vapid(&key_pair, "mailto:john.doe@example.com");

            // 4. Send it via an HTTP client
            let message = "test123".as_bytes().to_vec();

            let request = builder.build(message).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Build failed: {}", e),
                )
            })?;

            let reqwest_request = reqwest::Request::try_from(request).map_err(|e| {
                (
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to build request: {}", e),
                )
            })?;
            let client = reqwest::Client::new();
            let response = client.execute(reqwest_request).await.map_err(|e| {
                (
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to send request: {}", e),
                )
            })?;

            let status = response.status();
            let response_text = response.text().await.map_err(|e| {
                (
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR, // Or a specific code
                    format!("Failed to read response body: {}", e),
                )
            })?;
            println!("Status: {}", status);
            println!("Text: {}", response_text);
            let derived_public_key =
                Base64UrlUnpadded::encode_string(&key_pair.public_key().to_bytes());
            println!("Derived: {}", derived_public_key);
            println!("Hardcoded: {}", vapid_public_key);
        }
        let response = ApiResponse {
            message: "Broadcast complete".to_string(),
        };
        Ok((StatusCode::OK, Json(response)))
    } else {
        let response = ApiResponse {
            message: "User limit reached".to_string(),
        };
        Ok((StatusCode::OK, Json(response)))
    }
}

// Helpers for file I/O
async fn read_db() -> Db {
    let data = fs::read_to_string(DB_PATH)
        .await
        .unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&data).unwrap_or_default()
}

async fn write_db(db: Db) {
    let data = serde_json::to_string_pretty(&db).unwrap();
    fs::write(DB_PATH, data)
        .await
        .expect("Failed to write to disk");
}

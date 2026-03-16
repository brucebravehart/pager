use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
// use axum_server::tls_rustls::RustlsConfig; // Import TLS config
use dotenvy::dotenv;
use rustls_acme::{caches::DirCache, AcmeConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;
use web_push::*;

// Matches your JavaScript payload
#[derive(Serialize, Deserialize, Default)]
struct Db {
    usernames: Vec<String>,
    sub_objs: Vec<Value>,
}

const DB_PATH: &str = "data.json";

#[tokio::main]
async fn main() {
    // get env vars
    dotenv().ok();

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

    let app = Router::new()
        .route("/users", get(get_users))
        .route("/register_user", post(register_user))
        .route("/save-subscription", post(send_push));
    // Add CORS so your frontend can actually talk to it
    //.layer(cors);

    // Load your SSL Certificates
    // You need 'cert.pem' and 'key.pem' in your project folder
    /*let config = RustlsConfig::from_pem_file("cert.pem", "key.pem")
    .await
    .expect("Failed to load certificates. Do they exist?");*/

    let config = AcmeConfig::new(["pagerabcde.duckdns.com"]) // Replace with your domain
        .contact(["mailto:asdf@gmail.com"]) // Replace with your email
        .cache(DirCache::new(PathBuf::from("./rustls_acme_cache")))
        .directory_lets_encrypt(true); // Use production Let's Encrypt

    tokio::spawn(async move {
        loop {
            match state.next().await {
                Some(Ok(event)) => println!("ACME Event: {:?}", event),
                Some(Err(e)) => eprintln!("ACME Error: {:?}", e),
                None => break,
            }
        }
    });

    // Binding to port 443 requires sudo on Linux
    let addr = SocketAddr::from(([0, 0, 0, 0], 443));
    println!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        acceptor.into_make_service_with_connect_info::<()>(app),
    )
    .await
    .unwrap(); // old axum setup without tls

    /*axum_server::bind_rustls(addr, config)
    .serve(app.into_make_service())
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
    let mut db = read_db().await;
    if db.usernames.len() < 20 {
        let name = payload["name"].as_str().unwrap_or("Unknown").to_string();
        let sub_obj = payload["subObj"].clone();
        println!("{}", name);
        db.usernames.push(name);
        db.sub_objs.push(sub_obj);
        write_db(db).await;
        (StatusCode::OK, "User registered")
    } else {
        (StatusCode::BAD_REQUEST, "User limit reached")
    }
}

// POST /send-push
async fn send_push(Json(payload): Json<Value>) -> Result<&'static str, (StatusCode, String)> {
    let vapid_public_key =
        "BFDpLKw1c7dzDfr70rgdWMYI3v6wNX5WXbOxbSqBwzyEL7Md_bWzEblNo8D1s2mmOwNVhfpndrjI_MQQmJda58E";
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

        let client = IsahcWebPushClient::new()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for (i, sub) in db.sub_objs.iter().enumerate() {
            if i == skip_idx {
                continue; // Skip the person who triggered the push
            }

            // This is where you call the actual push logic for each 'sub'
            println!("Sending notification to user: {}", db.usernames[i]);

            // decode sub_obj
            let subscription_info: SubscriptionInfo =
                serde_json::from_value(sub.clone()).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to parse subscription from DB: {}", e),
                    )
                })?;

            // build signature
            let mut sig_builder =
                VapidSignatureBuilder::from_base64(&vapid_private_key, &subscription_info).unwrap();

            // Some push services require a contact email
            sig_builder.add_claim("sub", "mailto:admin@yourdomain.com");
            let signature = sig_builder.build().unwrap();

            // 3. Construct the message
            let mut builder = WebPushMessageBuilder::new(&subscription_info);
            builder.set_vapid_signature(signature);
            builder.set_payload(
                ContentEncoding::Aes128Gcm,
                "Hello from your VPS!".as_bytes(),
            );

            // 4. Send it via an HTTP client
            let message = builder.build().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Build failed: {}", e),
                )
            })?;
            client.send(message).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Send failed: {}", e),
                )
            })?;
        }
        Ok("Broadcast complete")
    } else {
        Ok("Sender not recognized. No push sent.")
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

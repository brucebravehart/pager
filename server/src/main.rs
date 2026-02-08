use axum::{
    routing::{get, post},
    Json, Router,
};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::net::SocketAddr;
use tokio::fs;
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

    let app = Router::new()
        .route("/users", get(get_users))
        .route("/register_user", post(register_user))
        .route("/save-subscription", post(send_push))
        // Add CORS so your frontend can actually talk to it
        .layer(CorsLayer::permissive());

    // Binding to port 443 requires sudo on Linux
    let addr = SocketAddr::from(([0, 0, 0, 0], 443));
    println!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// GET /users
async fn get_users() -> Json<Vec<User>> {
    let db = read_db().await;
    Json(db.usernames)
}

// POST /register_user
async fn register_user(Json(payload): Json<Value>) -> &'static str {
    let mut db = read_db().await;
    if db.usernames.len() < 20 {
        let name = payload["name"].as_str().unwrap_or("Unknown").to_string();
        let sub_obj = payload["subObj"];
        db.usernames.push(name);
        db.sub_objs.push(sub_obj);
        write_db(db).await;
        "User registered"
    } else {
        "User limit reached"
    }
}

// POST /send-push
async fn send_push(Json(payload): Json<Value>) -> &'static str {
    let vapid_public_key =
        "BFDpLKw1c7dzDfr70rgdWMYI3v6wNX5WXbOxbSqBwzyEL7Md_bWzEblNo8D1s2mmOwNVhfpndrjI_MQQmJda58E";
    let vapid_private_key = env::var("VAPID_PRIVATE_KEY").expect("VAPID_PRIVATE_KEY must be set");

    let db = read_db().await;
    let usernames = &db.usernames;
    let sub_objs = &db.sub_objs;

    let trigger_user = payload["name"].as_str().unwrap_or("Unknown").to_string();
    let trigger_sub_obj = payload["subObj"];

    let index = sub_objs
        .iter()
        .position(|json| json["endpoint"].as_str() == trigger_sub_obj["endpoint"].as_str());

    // Loop through and send to everyone else
    if let Some(skip_idx) = sender_index {
        println!(
            "Sender identified as index {}. Broadcasting to others...",
            skip_idx
        );

        for (i, sub) in db.sub_objs.iter().enumerate() {
            if i == skip_idx {
                continue; // Skip the person who triggered the push
            }

            // This is where you call the actual push logic for each 'sub'
            println!("Sending notification to user: {}", db.usernames[i]);

            // decode sub_obj
            let subscription_info: SubscriptionInfo = serde_json::from_value(sub_val.clone())
                .map_err(|_| WebPushError::InvalidSubscriptionInfo)?;

            // build signature
            let mut sig_builder =
                VapidSignatureBuilder::from_base64(&vapid_private_key, &subscription_info).unwrap();

            // Some push services require a contact email
            sig_builder.add_claim("sub", "mailto:admin@yourdomain.com");
            let signature = sig_builder.build().unwrap();

            // 3. Construct the message
            let mut builder = WebPushMessageBuilder::new(&subscription_info)?;
            builder.set_vapid_signature(signature);
            builder.set_payload(
                ContentEncoding::Aes128Gcm,
                "Hello from your VPS!".as_bytes(),
            );

            // 4. Send it via an HTTP client
            let client = WebPushClient::new()?;
            client.send(builder.build()?).await?;
        }
        "Broadcast complete"
    } else {
        "Sender not recognized. No push sent."
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

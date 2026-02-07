use axum::{
    routing::{get, post},
    Json, Router,
};
use web_push::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::fs;
use tower_http::cors::CorsLayer;

// Matches your JavaScript payload
#[derive(Serialize, Deserialize, Clone)]
struct User {
    name: String,
    #[serde(rename = "subObj")]
    sub_obj: serde_json::Value,
}

#[derive(Serialize, Deserialize, Default)]
struct Db {
    users: Vec<User>,
    subscriptions: Vec<serde_json::Value>,
}

const DB_PATH: &str = "data.json";

#[tokio::main]
async fn main() {
    // Initialize DB file if it doesn't exist
    if fs::metadata(DB_PATH).await.is_err() {
        let initial_db = serde_json::to_string(&Db::default()).unwrap();
        fs::write(DB_PATH, initial_db).await.expect("Failed to create DB file");
    }

    let app = Router::new()
        .route("/users", get(get_users))
        .route("/register_user", post(register_user))
        .route("/save-subscription", post(send_push))
        // Add CORS so your frontend can actually talk to it
        .layer(CorsLayer::permissive());

    // Binding to port 80 requires sudo on Linux
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
    println!("Server running on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// GET /users
async fn get_users() -> Json<Vec<User>> {
    let db = read_db().await;
    Json(db.users)
}

// POST /register_user
async fn register_user(Json(payload): Json<User>) -> &'static str {
    let mut db = read_db().await;
    if db.users.len() < 20 {
        db.users.push(payload);
        write_db(db).await;
        "User registered"
    } else {
        "User limit reached"
    }
}

// POST /send-push
async fn send_push(Json(user): Json<serde_json::Value>) -> &'static str {
    const VAPID_PUBLIC_KEY = "BFDpLKw1c7dzDfr70rgdWMYI3v6wNX5WXbOxbSqBwzyEL7Md_bWzEblNo8D1s2mmOwNVhfpndrjI_MQQmJda58E"
    const VAPID_PRIVATE_KEY = "JTxB87ScnjG6HYHsl27b6rA2toFs7wVV1QIiYsOdWwU"
    let mut db = read_db().await;
    db.subscriptions.push(sub);
    write_db(db).await;
    "Subscription saved"
}

// Helpers for file I/O
async fn read_db() -> Db {
    let data = fs::read_to_string(DB_PATH).await.unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&data).unwrap_or_default()
}

async fn write_db(db: Db) {
    let data = serde_json::to_string_pretty(&db).unwrap();
    fs::write(DB_PATH, data).await.expect("Failed to write to disk");
}
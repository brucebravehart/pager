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

    // Binding to port 80 requires sudo on Linux
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
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
async fn send_push(Json(user): Json<serde_json::Value>) -> &'static str {
    let vapid_public_key =
        "BFDpLKw1c7dzDfr70rgdWMYI3v6wNX5WXbOxbSqBwzyEL7Md_bWzEblNo8D1s2mmOwNVhfpndrjI_MQQmJda58E";
    let vapid_private_key = env::var("VAPID_PRIVATE_KEY");

    let db = read_db().await;
    let users = db.users;
    let subscriptions = db.subscriptions;

    let index = subscriptions.iter().position();
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

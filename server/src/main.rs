use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64ct::{Base64UrlUnpadded, Encoding};
use dotenvy::dotenv;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::net::SocketAddr;
use tokio::fs;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::normalize_path::NormalizePathLayer;
use web_push_native::{
    jwt_simple::algorithms::ES256KeyPair, p256::PublicKey, Auth, WebPushBuilder,
};

const USER_LIMIT: usize = 20;
const VAPID_SUBJECT: &str = "mailto:john.doe@example.com";
const VAPID_PUBLIC_KEY: &str =
    "BDspVj_KfBb-AOxX8zg69l74H_YRwHXr_D6mk0gdqxKy0UOqFRn1wJeD5JIvgGiSvtbq9feY0J0O4ytzaUzWxJU";

// Local cache kept for compatibility with older client state.
#[derive(Serialize, Deserialize, Default)]
struct Db {
    usernames: Vec<String>,
    sub_objs: Vec<Value>,
}

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

const DB_PATH: &str = "data.json";

#[derive(Serialize)]
struct ApiResponse {
    message: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // get env vars

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let options = sqlx::postgres::PgConnectOptions::new()
        .host("ep-quiet-art-alcgs14k.c-3.eu-central-1.aws.neon.tech")
        .username("neondb_owner")
        .password(&database_url)
        .database("neondb")
        .ssl_mode(sqlx::postgres::PgSslMode::Require);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("Failed to connect to DB");
    let state = AppState { db: pool };

    if fs::metadata(DB_PATH).await.is_err() {
        let initial_db = serde_json::to_string(&Db::default()).unwrap();
        fs::write(DB_PATH, initial_db)
            .await
            .expect("Failed to create DB file");
    }

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
        .route("/send-push", post(send_push))
        .route("/status", get(return_status))
        .with_state(state)
        .layer(cors);

    let app = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        .service(router);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, tower::make::Shared::new(app))
        .await
        .unwrap();
}

// GET /users
async fn get_users(State(state): State<AppState>) -> Result<Json<Vec<String>>, String> {
    let rows = read_db_remote(&state.db).await.map_err(|e| e.to_string())?;
    let usernames: Vec<String> = rows.iter().map(|row| row.1.clone()).collect();

    Ok(Json(usernames))
}

// POST /register_user
async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> impl axum::response::IntoResponse {
    let mut db = read_db().await;
    if db.usernames.len() < USER_LIMIT {
        let name = payload["name"].as_str().unwrap_or("Unknown").to_string();
        let sub_obj = payload["subObj"].clone();

        let result = write_db_remote(&state.db, name.clone(), sub_obj.clone()).await;

        db.usernames.push(name.clone());
        db.sub_objs.push(sub_obj.clone());
        write_db(db).await;

        if result.is_err() {
            let response = ApiResponse {
                message: "DB Error".to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        } else {
            let response = ApiResponse {
                message: "User registered".to_string(),
            };
            (StatusCode::OK, Json(response))
        }
    } else {
        let response = ApiResponse {
            message: "User limit reached".to_string(),
        };
        (StatusCode::BAD_REQUEST, Json(response))
    }
}

// POST /send-push
async fn send_push(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let vapid_private_key = env::var("VAPID_PRIVATE_KEY").expect("VAPID_PRIVATE_KEY must be set");

    let rows = read_db_remote(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let sub_objs: Vec<Value> = rows.iter().map(|row| row.2.clone()).collect();

    let trigger_user = payload["name"].as_str().unwrap_or("Unknown").to_string();
    let trigger_sub_obj = payload["subObj"].clone();

    let index = sub_objs
        .iter()
        .position(|json| json["endpoint"].as_str() == trigger_sub_obj["endpoint"].as_str());

    if let Some(skip_idx) = index {
        println!(
            "Sender identified as index {}. Broadcasting to others...",
            skip_idx
        );

        let decoded_bytes = Base64UrlUnpadded::decode_vec(&vapid_private_key)
            .expect("VAPID private key is not valid base64");
        let key_pair = ES256KeyPair::from_bytes(&decoded_bytes)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for (i, sub) in sub_objs.iter().enumerate() {
            if i == skip_idx {
                continue;
            }

            println!("Sending notification to user: {}", rows[i].1.clone());

            let current_sub_obj = sub.clone();

            let endpoint: &str = current_sub_obj
                .get("endpoint")
                .and_then(|v| v.as_str())
                .unwrap();
            let p256dh: &str = current_sub_obj
                .get("p256dh")
                .and_then(|v| v.as_str())
                .unwrap();
            let auth: &str = current_sub_obj
                .get("auth")
                .and_then(|v| v.as_str())
                .unwrap();

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

            let builder =
                WebPushBuilder::new(endpoint, public_key, Auth::clone_from_slice(&auth_bytes))
                    .with_vapid(&key_pair, VAPID_SUBJECT);

            let payload = serde_json::json!({
                "title": "GROSS ALARM!",
                "body": format!("ausgelöst durch {}", trigger_user),
                "url": "/"
            });
            let message = payload.to_string().into_bytes();

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
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read response body: {}", e),
                )
            })?;
            println!("Status: {}", status);
            println!("Text: {}", response_text);

            if !status.is_success() {
                let _ = delete_db_remote(&state.db, rows[i].0).await;
            }

            let pub_key_bytes = key_pair.public_key().to_bytes();
            let standard_key = p256::PublicKey::from_sec1_bytes(&pub_key_bytes).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Key bridge failed: {}", e),
                )
            })?;
            let encoded_point = standard_key.to_encoded_point(false);
            let derived_public_key = Base64UrlUnpadded::encode_string(encoded_point.as_bytes());
            println!("Derived: {}", derived_public_key);
            println!("Hardcoded: {}", VAPID_PUBLIC_KEY);
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

async fn return_status() -> StatusCode {
    StatusCode::OK
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

async fn write_db_remote(
    pool: &PgPool,
    user_name: String,
    sub_obj: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users (username, subscription_json) VALUES ($1, $2)")
        .bind(user_name)
        .bind(sub_obj)
        .execute(pool)
        .await?;

    Ok(())
}

async fn read_db_remote(pool: &PgPool) -> Result<Vec<(i32, String, Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (i32, String, Value)>(
        "SELECT id, username, subscription_json FROM users",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

async fn delete_db_remote(pool: &PgPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

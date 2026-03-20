mod ai;
mod db;
mod handlers;
mod models;

use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    let database_url = "sqlite:./data/lingua.db";
    std::fs::create_dir_all("./data").expect("Failed to create data directory");

    let pool = db::create_pool(database_url).await;
    db::migrate(&pool).await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/contents", get(handlers::list_contents))
        .route("/api/contents", post(handlers::create_content))
        .route("/api/contents/:id", get(handlers::get_content))
        .route("/api/contents/:id", delete(handlers::delete_content))
        .route("/api/contents/:id", axum::routing::put(handlers::update_content))
        .route("/api/contents/:id/translate", post(handlers::translate_content))
        .route("/api/contents/:id/summary", post(handlers::summarize_content))
        .route("/api/sentences/:id", axum::routing::put(handlers::update_sentence))
        .layer(cors)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

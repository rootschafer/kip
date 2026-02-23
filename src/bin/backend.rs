//! Kip Web Backend - Actix Web server serving Dioxus web app

use actix_web::{get, web, App, HttpResponse, HttpServer};
use actix_web_flash_messages::IncomingFlashMessages;
use actix_dioxus_serve::{
    DioxusAssetsConfig, 
    serve_dioxus_assets, 
    serve_dioxus_html,
    include_dioxus_html,
};

// Embed the Dioxus HTML at compile time
include_dioxus_html!(INDEX_HTML, ".", "/");

#[get("/")]
async fn index(flash: IncomingFlashMessages) -> HttpResponse {
    serve_dioxus_html(INDEX_HTML, flash).await.into()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting Kip backend on http://127.0.0.1:8080");

    let assets = serve_dioxus_assets(&DioxusAssetsConfig::new("."));

    HttpServer::new(move || {
        App::new()
            .service(index)
            .service(assets)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

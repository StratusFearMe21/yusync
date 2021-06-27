use actix_web::{http::StatusCode, web, App, HttpResponse, HttpServer};
use async_std::io::prelude::WriteExt;
use ed25519_dalek::{Keypair, Signature};
use futures::StreamExt;
use serde::Deserialize;

#[derive(Deserialize)]
struct ServerMessage {
    message: Vec<u8>,
    signature: Signature,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .route("/rev", web::get().to(rev))
            .route("/upload", web::post().to(upload))
            .route("/download", web::get().to(download))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn rev() -> HttpResponse {
    let rev = async_std::fs::read_to_string("rev")
        .await
        .unwrap_or_else(|_| {
            let to_write = "1".to_string();
            std::fs::write("rev", &to_write).unwrap();
            to_write
        });
    HttpResponse::Ok().body(rev)
}

async fn download() -> actix_web::HttpResponse {
    HttpResponse::Ok().body(async_std::fs::read("file").await.unwrap())
}

async fn upload(payload: web::Payload) -> actix_web::HttpResponse {
    async_std::fs::write("file", test(payload).await.unwrap())
        .await
        .unwrap();
    async_std::fs::write(
        "rev",
        (async_std::fs::read_to_string("rev")
            .await
            .unwrap_or_else(|_| {
                let to_write = "0".to_string();
                std::fs::write("rev", &to_write).unwrap();
                to_write
            })
            .parse::<u32>()
            .unwrap()
            + 1)
        .to_string(),
    )
    .await
    .unwrap();
    HttpResponse::Ok().finish()
}

async fn test(mut payload: web::Payload) -> Result<Vec<u8>, HttpResponse> {
    let mut bytes = Vec::new();
    while let Some(item) = payload.next().await {
        bytes.write(&*item.unwrap()).await.unwrap();
    }
    let server_message: ServerMessage = bincode::deserialize(bytes.as_slice()).unwrap();
    let keypair = Keypair::from_bytes(std::fs::read("keyfile").unwrap().as_slice()).unwrap();
    if keypair
        .verify(server_message.message.as_slice(), &server_message.signature)
        .is_ok()
    {
        Ok(server_message.message)
    } else {
        println!("forbidden");
        Err(HttpResponse::new(StatusCode::FORBIDDEN))
    }
}

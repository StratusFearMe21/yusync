use std::borrow::Borrow;

use actix_web::{web, App, HttpResponse, HttpServer};
use async_std::io::prelude::WriteExt;
use ed25519_dalek::{PublicKey, Signature, Verifier};
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
            .route("/request", web::post().to(request))
            .route("/upload", web::post().to(upload))
            .route("/init", web::post().to(init))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn request(payload: web::Payload) -> HttpResponse {
    match match verify_payload(payload).await {
        Ok(s) => String::from_utf8(s).unwrap(),
        Err(c) => return c,
    }
    .borrow()
    {
        "file" => HttpResponse::Ok().body(async_std::fs::read("file").await.unwrap()),
        "rev" => HttpResponse::Ok().body(async_std::fs::read("rev").await.unwrap()),
        _ => HttpResponse::InternalServerError().finish(),
    }
}

async fn init(mut payload: web::Payload) -> HttpResponse {
    if std::path::Path::new("keyfile").exists() {
        return HttpResponse::Forbidden().body("Server already initialized");
    } else {
        let mut file = async_std::fs::File::create("keyfile").await.unwrap();
        while let Some(item) = payload.next().await {
            file.write(&*item.unwrap()).await.unwrap();
        }

        async_std::fs::write("rev", "0").await.unwrap();
        async_std::fs::write("file", "").await.unwrap();

        HttpResponse::Ok().finish()
    }
}

async fn upload(payload: web::Payload) -> actix_web::HttpResponse {
    let verified_payload = match verify_payload(payload).await {
        Ok(s) => s,
        Err(c) => return c,
    };
    async_std::fs::write("file", verified_payload)
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

async fn verify_payload(mut payload: web::Payload) -> Result<Vec<u8>, HttpResponse> {
    let mut bytes = Vec::new();
    while let Some(item) = payload.next().await {
        bytes.write(&*item.unwrap()).await.unwrap();
    }
    let server_message: ServerMessage = bincode::deserialize(bytes.as_slice()).unwrap();
    let keypair = PublicKey::from_bytes(std::fs::read("keyfile").unwrap().as_slice()).unwrap();
    if keypair
        .verify(server_message.message.as_slice(), &server_message.signature)
        .is_ok()
    {
        Ok(server_message.message)
    } else {
        Err(HttpResponse::Forbidden().body("Access denied, invalid signature"))
    }
}

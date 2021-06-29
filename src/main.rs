use std::{borrow::Borrow, io::Write};

use actix_web::{web, App, HttpResponse, HttpServer};
use crypto_box::{aead::Aead, Box, PublicKey, SecretKey};
use futures::StreamExt;
use once_cell::sync::Lazy;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ServerMessage {
    message: Vec<u8>,
    nonce: [u8; 24],
}

static KEYBOX: Lazy<Box> = Lazy::new(|| {
    Box::new(
        &pubkey_slice(std::fs::read("client_public").unwrap().as_slice()),
        &secretkey_slice(std::fs::read("server_private").unwrap().as_slice()),
    )
});

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
    match match decrypt_payload(payload).await {
        Ok(s) => String::from_utf8(s).unwrap(),
        Err(c) => return c,
    }
    .borrow()
    {
        "file" => HttpResponse::Ok().body(encrypt_payload(std::fs::read("file").unwrap()).unwrap()),
        "rev" => HttpResponse::Ok().body(encrypt_payload(std::fs::read("rev").unwrap()).unwrap()),
        _ => HttpResponse::InternalServerError().finish(),
    }
}

async fn init(mut payload: web::Payload) -> HttpResponse {
    if std::path::Path::new("server_private").exists() {
        return HttpResponse::Forbidden().body("Server already initialized");
    } else {
        let mut file = std::fs::File::create("client_public").unwrap();
        while let Some(item) = payload.next().await {
            file.write(&*item.unwrap()).unwrap();
        }

        std::fs::write("rev", "0").unwrap();
        std::fs::write("file", "").unwrap();

        let server_key = SecretKey::generate(&mut OsRng);

        std::fs::write("server_private", server_key.to_bytes()).unwrap();

        HttpResponse::Ok().body(server_key.public_key().as_bytes().to_vec())
    }
}

async fn upload(payload: web::Payload) -> actix_web::HttpResponse {
    let verified_payload = match decrypt_payload(payload).await {
        Ok(s) => s,
        Err(c) => return c,
    };
    std::fs::write("file", verified_payload).unwrap();
    std::fs::write(
        "rev",
        (std::fs::read_to_string("rev")
            .unwrap()
            .parse::<u32>()
            .unwrap()
            + 1)
        .to_string(),
    )
    .unwrap();
    HttpResponse::Ok().finish()
}

async fn decrypt_payload(mut payload: web::Payload) -> Result<Vec<u8>, HttpResponse> {
    let mut bytes = Vec::new();
    while let Some(item) = payload.next().await {
        bytes.write(&*item.unwrap()).unwrap();
    }
    let deserialized_payload: ServerMessage = bincode::deserialize(bytes.as_slice()).unwrap();
    match KEYBOX.decrypt(
        &deserialized_payload.nonce.into(),
        deserialized_payload.message.as_slice(),
    ) {
        Ok(d) => Ok(d),
        Err(_) => Err(HttpResponse::Forbidden().body("Access denied, invalid signature")),
    }
}

fn encrypt_payload(message: Vec<u8>) -> Result<Vec<u8>, HttpResponse> {
    let nonce = crypto_box::generate_nonce(&mut OsRng);
    match bincode::serialize(&ServerMessage {
        nonce: nonce.into(),
        message: KEYBOX.encrypt(&nonce, message.as_slice()).unwrap(),
    }) {
        Ok(d) => Ok(d),
        Err(_) => Err(HttpResponse::InternalServerError().body("Could not encrypt payload")),
    }
}

fn pubkey_slice(s: &[u8]) -> PublicKey {
    let mut a: [u8; 32] = Default::default();
    a.copy_from_slice(s);
    PublicKey::from(a)
}

fn secretkey_slice(s: &[u8]) -> SecretKey {
    let mut a: [u8; 32] = Default::default();
    a.copy_from_slice(s);
    SecretKey::from(a)
}

use actix_web::{web, App, HttpResponse, HttpServer};
use async_std::io::prelude::WriteExt;
use futures::StreamExt;

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

async fn upload(mut payload: web::Payload) -> actix_web::HttpResponse {
    let mut file = async_std::fs::File::create("file").await.unwrap();
    while let Some(item) = payload.next().await {
        file.write(&*item.unwrap()).await.unwrap();
    }
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

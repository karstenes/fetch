mod searchengine;

use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{
    middleware, web, App, HttpResponse, HttpServer, Responder, Result, HttpRequest, 
};

use futures::FutureExt;
use serde::Deserialize;

use std::time::Duration;

use tokio;

use futures::future::try_join_all;

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn metasearch(query: web::Query<SearchQuery>) -> impl Responder {
    let ddg = searchengine::duckduckgo::search(&query.q, Duration::new(5,0)).boxed();
    let goog = searchengine::google::search(&query.q, Duration::new(5,0)).boxed();

    let futs = vec![ddg, goog];

    let result = try_join_all(futs).await;

    let response = match result {
        Ok(o) => format!("{:?}", o),
        Err(e) => panic!("{:}", e),
    };

    HttpResponse::Ok().body(response)
    //HttpResponse::Ok().body(searchengine::google::search(&query.q, Duration::new(5,0)).await.expect("thing"))
}

async fn p404(_req: HttpRequest) -> Result<fs::NamedFile> {
    println!("{}", _req.uri().path());
    Ok(fs::NamedFile::open("static/404.html")?.set_status_code(StatusCode::NOT_FOUND))
}

async fn index() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/index.html")?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .service(
                web::scope("/fetch")
                    .route("", web::get().to(index))
                    .service(web::scope("/search").route("", web::to(metasearch)))
                    .service(fs::Files::new("/static", "static").show_files_listing())
            )
            .default_service(web::route().to(p404))
            
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

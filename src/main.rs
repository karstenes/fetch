mod searchengine;

use std::convert::TryInto;
use std::time::Duration;

use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder, Result, HttpRequest};

use futures::FutureExt;
use searchengine::SearchListing;
use serde::{Serialize, Deserialize};

use futures::future::try_join_all;

use tinytemplate::TinyTemplate;

static RESULT: &str = 
r##"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta http-equiv="X-UA-Compatible" content="IE=edge">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="stylesheet" href="static/reset.css" />
    <link rel="stylesheet" href="static/style.css" />
    <title>Results</title>
    <style>
        .results__header \{
            display: flex;
            justify-content: space-between;
            align-items: center;
            max-width: 60vw;
            gap: 3rem;
        }
        .results__header .logo \{
            width: 30%;
        }
        .results__header .results__search-bar \{
            width: 70%;
        }
        .results__results \{
            padding: 1rem 1rem 1rem 22vw;
            max-width: 60vw;

            display: flex;
            flex-direction: column;
            gap: 1.5rem;
        }
        .result__link \{
            display: flex;
            flex-direction: column;
        }
        .result__title \{
            font-size: 1.5rem;
            color: #99ccff;
        }
        .result__title:hover \{
            text-decoration: underline;
        }
        .result__url \{
            font-size: .875rem;
        }
        .result__desc \{
            margin-top: 0.25rem;
        }
        .results__footer \{
            padding: 1rem 1rem 1rem 22vw;
            max-width: 60vw;
        }
        .results__paging \{
            margin: 0 auto;
            display: flex;
            justify-content: center;
            gap: 1rem;
        }
        .paging__nav \{
            color: #99ccff;
        }
        .paging__nav:hover \{
            text-decoration: underline;
        }
        .paging__nav.paging__active,
        .paging__nav.paging__active:hover \{
            color: unset;
            text-decoration: unset;
        }
    </style>
</head>
<body class="results">
    <header class="results__header">
        <h1 class="logo"><a href="/fetch">[Fe]TCH</a></h1>
        <form action="result" class="results__search-bar">
          <input
            type="text"
            name="q"
            id="q"
            placeholder="Enter search query"
            class="results__input"
          />
          <input type="submit" value="🔍" class="results__submit" />
        </form>
    </header>
    <main class="results__results">
{{ for listing in results }}
    <div class="result">
        <a href="{listing.url}" class="result__link">
            <span class="result__title">
                {listing.title}
            </span>
            <span class="result__url">
                {listing.url}
            </span>
        </a>
        <p class="result__desc">
            {listing.description}
        </p>
    </div>
{{ endfor }}
</main>
<footer class="results__footer">
<div class="results__paging">
    <a href="#" class="paging__nav paging__prev">◀ Previous</a>
    <a href="#" class="paging__nav paging__num paging__active">1</a>
    <a href="#" class="paging__nav paging__num">2</a>
    <a href="#" class="paging__nav paging__num">3</a>
    <a href="#" class="paging__nav paging__num">4</a>            
    <a href="#" class="paging__nav paging__num">5</a>
    <a href="#" class="paging__nav paging__num">6</a>
    <a href="#" class="paging__nav paging__num">7</a>                        
    <a href="#" class="paging__nav paging__num">8</a>                        
    <a href="#" class="paging__nav paging__num">9</a>                                    
    <a href="#" class="paging__nav paging__next">Next ▶</a>
</div>
</footer>
</body>
</html>"##;

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
struct Context<'a> {
    results: &'a [SearchListing]
}

async fn metasearch(query: web::Query<SearchQuery>) -> impl Responder {
    let ddg = searchengine::duckduckgo::search(&query.q, Duration::new(5,0)).boxed();
    //let goog = searchengine::google::search(&query.q, Duration::new(5,0)).boxed();

    let futs = vec![ddg];

    let result = try_join_all(futs).await;

    let response = match result {
        Ok(o) => o,
        Err(e) => panic!("{:}", e),
    };

    let mut tt = TinyTemplate::new();

    tt.add_template("result", RESULT).unwrap();

    let rendered = tt.render("result", &Context{results: &response[0].results}).unwrap();

    HttpResponse::Ok().body(rendered)
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

mod searchengine;
use std::time::Duration;

use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder, Result, HttpRequest};

use futures::FutureExt;
use searchengine::{SearchListing, Search};
use serde::{Serialize, Deserialize};

use futures::future::join_all;

use tinytemplate::TinyTemplate;

use rayon::prelude::*;

use log::{debug, error, log_enabled, info, Level};

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
    <title>[Fe]TCHED {title}</title>
</head>
<body class="results">
    <header class="results__header">
        <h1 class="logo"><a href="/">[Fe]TCH</a></h1>
        <form action="search" class="results__search-bar">
          <input
            type="text"
            name="q"
            id="q"
            placeholder="Enter search query"
            class="results__input"
            value="{query}"
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
            <div class="result__meta">
                <span class="result__url">{listing.url}</span>
                <span class="result__source">{listing.sources}</span>
            </div>
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
    title: &'a String,
    results: &'a [Listing],
    query: &'a str
}

#[derive(Debug, Clone, Serialize)]
pub struct Listing {
    pub title: String,
    pub url: String,
    pub description: String,
    pub sources: String,
    pub quality: u8
}

async fn resolve_collisions(listings: Vec<Option<Search>>) -> Vec<Listing> {
    let mut combined = Vec::new();

    for o in listings {
        match o {
            Some(x) => {
                for y in x.results {
                    let s = combined.iter_mut().find(|r: &&mut SearchListing| -> bool {(*(*r)).url == y.url});
                    match s {
                        Some(val) => {
                            val.sources.push(y.sources[0].clone());
                            val.quality += y.quality;
                        }
                        None => {
                            
                            combined.push(y)
                        }
                    }
                }
            }
            None => {
                continue;
            }
        };
    }
    let mut combinednew: Vec<Listing> = combined.par_iter()
    .map(|x| Listing{title: x.title.clone(), url: x.url.clone(), description: x.description.clone(), quality: x.quality, sources: 
        x.sources.par_iter()
        .map(|y| format!("{:?}", y)).
        collect::<Vec<String>>().join(" ")})
        .collect();
    combinednew.par_sort_unstable_by(|a,b| a.quality.cmp(&b.quality).reverse());
    combinednew
}

async fn metasearch(query: web::Query<SearchQuery>) -> impl Responder {
    let start = tokio::time::Instant::now();

    let futs = vec![
        searchengine::duckduckgo::search(&query.q, Duration::new(5,0)).boxed(),
        searchengine::google::search(&query.q, Duration::new(5,0)).boxed(),
        searchengine::bing::search(&query.q, Duration::new(5,0)).boxed(),
        searchengine::brave::search(&query.q, Duration::new(5,0)).boxed()
        ];

    let result = join_all(futs).await;

    let response = result.iter().filter_map(|x|{
        match x {
            Ok(o) => {
                Some(o.clone())
            }
            Err(e) => {
                panic!("{:}", e)
            }
        }    
    }).collect();

    let render = tokio::time::Instant::now();

    let mut tt = TinyTemplate::new();

    tt.add_template("result", RESULT).unwrap();
    
    let mut torender = resolve_collisions(response).await;

    torender.truncate(20);

    let rendered = tt.render("result", &Context{title: &query.q, results: &torender, query: &query.q}).unwrap();
    

    info!("Total time: {}, Fetch time: {}, Render time: {}", start.elapsed().as_secs_f32(), start.elapsed().as_secs_f32()-render.elapsed().as_secs_f32(), render.elapsed().as_secs_f32());

    HttpResponse::Ok().body(rendered)
    //HttpResponse::Ok().body(format!("{:?}", response))
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
    env_logger::init();    

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

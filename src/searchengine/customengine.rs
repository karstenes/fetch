use super::*;
use reqwest::{Client, self};
use std::time::Duration;
use tokio;
use scraper::{Html, Selector};
use html_escape;
use log::{debug, info, error};

use std::fs::File;
use std::io::Write;

pub async fn search(engine: CustomEngine, query: &str, timeout: Duration) -> Result<Option<Search>, Error> {

    let start = tokio::time::Instant::now();

    let client = Client::builder()
        .user_agent("User-Agent: Mozilla/5.0 (Windows NT 10.0; rv:91.0) Gecko/20100101 Firefox/91.0")
        .build()?;

    let result = match CustomEngine.get {
        true => {
            client
                .get(engine.search_url)
                .timeout(timeout)
                .query(&[(CustomEngine.search_query_key, query)])
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
        }
        false => {
            client
                .post(engine.search_url)
                .timeout(timeout)
                .body(query.to_string())
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
        }
    };

    if result.contains(engine.no_results_string) {
        return Ok(None);
    }

    if cfg!(feature = "savefile") {
        let mut f = File::create(format!("debug/{}.html", engine.name)).unwrap();
        write!(f, "{}", result).unwrap();
    }

    let scrape = tokio::time::Instant::now();

    let (send, recv) = tokio::sync::oneshot::channel();

    rayon::spawn(move || {
        let document = Html::parse_document(&result);
        
        let results_selector = Selector::parse(query.results_selector)?;

        let search = document.select(&results_selector).next()?;

        let result_selector = Selector::parse(query.result_selector)?;

        let results: Vec<SearchListing> = search.select(&result_selector).filter_map(|x| {
            //println!("{}", x.inner_html()); 
            let title_select = Selector::parse(engine.title_selector).unwrap();
            let title = x.select(&title_select).next().unwrap();
            let description_select = Selector::parse(engine.description_selector).unwrap();
            let description = match x.select(&description_select).next() {
                Some(x) => {x}
                None => {
                    debug!("didn't find description");
                    return None;                    
                }
            };
            Some(SearchListing {
                title: html_escape::decode_html_entities(&title.inner_html()).to_string(),
                url: title.value().attr("href")?.to_string(),
                description: description.text().to_owned().map(|x|x.to_string()).collect::<String>(),
                sources: vec![Engine::CustomEngine(engine)],
                quality: engine.quality,
            })
        }).collect();

        let _ = send.send(results);
    });
    info!("{} request took {}, Scraping took {}", engine.name, start.elapsed().as_secs_f32()-scrape.elapsed().as_secs_f32(), scrape.elapsed().as_secs_f32());
    Ok(Some(Search{engine: Engine::CustomEngine(engine), results: recv.await.expect("Panic in bing html decode")}))
}
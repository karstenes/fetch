use super::*;
use reqwest::{Client, self};
use std::time::Duration;
use tokio;
use scraper::{Html, Selector};
use html_escape;

use std::fs::File;
use std::io::Write;


const RESULT_PATH: &str = "div#results>div.snipper.fdb";
const URL_PATH: &str = "h2>a";
const DESCRIPTION_PATH: &str = "div.b_caption>p";
const TITLE_PATH: &str = "h2>a";

pub async fn search(query: &str, timeout: Duration) -> Result<Option<Search>, Error> {

    let start = tokio::time::Instant::now();

    let result = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/100.0.4896.75 Safari/537.36 Edg/100.0.1185.36")
        .build()?
        .get("https://search.brave.com/search")
        //.body(query.to_string())
        .timeout(timeout)
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    if result.contains(r#"class="b_no""#) {
        return Ok(None);
    }

    if cfg!(debug_assertions) {
        let mut f = File::create("debug/bing.html").unwrap();
        write!(f, "{}", result).unwrap();
    }

    let scrape  = tokio::time::Instant::now();

    let (send, recv) = tokio::sync::oneshot::channel();

    rayon::spawn(move || {
        let document = Html::parse_document(&result);       
        let result_selector = Selector::parse(RESULT_PATH).unwrap();

        let results: Vec<SearchListing> = document.select(&result_selector).filter_map(|x| {
            //println!("{}", x.inner_html()); 
            let title_select = Selector::parse(TITLE_PATH).unwrap();
            let title = x.select(&title_select).next().unwrap();
            let snippet_select = Selector::parse(DESCRIPTION_PATH).unwrap();
            let snippet = x.select(&snippet_select).next()?;
            let url_select = Selector::parse(URL_PATH).unwrap();
            let url = x.select(&url_select).next()?;
            Some(SearchListing {
                title: html_escape::decode_html_entities(&title.text().to_owned().map(|x|x.to_string()).collect::<String>()).to_string(),
                url: url.value().attr("href").unwrap().to_string(),
                description: snippet.text().to_owned().map(|x|x.to_string()).collect::<String>(),
                sources: vec![Engine::Brave],
                quality: 4,
            })
        }).collect();

        let _ = send.send(results);
    });
    println!("Brave request took {}, Scraping took {}", start.elapsed().as_secs_f32()-scrape.elapsed().as_secs_f32(), scrape.elapsed().as_secs_f32());
    Ok(Some(Search{engine: Engine::Brave, results: recv.await.expect("Panic in bing html decode")}))
}
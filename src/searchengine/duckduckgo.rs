use super::*;
use reqwest::{Client, self};
use std::time::Duration;
use tokio;
use scraper::{Html, Selector};

pub async fn search(query: &str, timeout: Duration) -> Result<Search, reqwest::Error> {

    let result = Client::builder()
        .user_agent("User-Agent: Mozilla/5.0 (Windows NT 10.0; rv:91.0) Gecko/20100101 Firefox/91.0")
        .build()?
        .post("https://html.duckduckgo.com/html")
        .body(query.to_string())
        .timeout(timeout)
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let (send, recv) = tokio::sync::oneshot::channel();

    rayon::spawn(move || {
        let document = Html::parse_document(&result);
        
        let links_selector = Selector::parse("#links").unwrap();
        let result_selector = Selector::parse(".result").unwrap();

        let links = document.select(&links_selector).next().unwrap();

        let results: Vec<SearchListing> = links.select(&result_selector).map(|x| { 
            let snippet_select = Selector::parse(".result__snippet").unwrap();
            let snippet = x.select(&snippet_select).next().unwrap();
            let title_select = Selector::parse(".result__a").unwrap();
            let title = x.select(&title_select).next().unwrap();
            SearchListing {
                title: title.inner_html(),
                url: title.value().attr("href").unwrap().to_string(),
                description: snippet.text().next().unwrap().to_string()
            }
        }).collect();

        let _ = send.send(results);
    });

    Ok(Search{engine: Engine::DuckDuckGo, results: recv.await.expect("Panic in duckduckgo html decode")})
}
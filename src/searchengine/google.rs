use super::*;
use reqwest::{Client, self};
use std::time::Duration;
use scraper::{Html, Selector, ElementRef};

const RESULT_PATH: &str = ".g";
const URL_PATH: &str = "div.yuRUbf>a";
const DESCRIPTION_PATH: &str = "div.VwiC3b";
const TITLE_PATH: &str = "h3";

pub async fn search(query: &str, timeout: Duration) -> Result<Search, reqwest::Error> {

    let result = Client::builder()
        .user_agent("User-Agent: Mozilla/5.0 (Windows NT 10.0; rv:91.0) Gecko/20100101 Firefox/91.0")
        .build()?
        .get("https://www.google.com/search")
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
        let result_selector = Selector::parse(RESULT_PATH).unwrap();
        
        let results: Vec<SearchListing> = document.select(&result_selector).map(|x| { 

            let snippet_select = Selector::parse(DESCRIPTION_PATH).unwrap();
            let link_select = Selector::parse(URL_PATH).unwrap();
            let title_select = Selector::parse(TITLE_PATH).unwrap();

            let snippet = x.select(&snippet_select).next().unwrap();
            let link = x.select(&link_select).next().unwrap();
            let title = x.select(&title_select).next().unwrap();

            SearchListing {
                title: title.text().to_owned().map(|x|x.to_string()).collect::<String>(),
                url: link.value().attr("href").unwrap().to_string(),
                description: snippet.text().to_owned().map(|x|x.to_string()).collect::<String>()
            }
        }).collect();

        let _ = send.send(results);
    });

    Ok(Search{engine: Engine::Google, results: recv.await.expect("Panic in duckduckgo html decode")})
    
    //Ok(result)
}
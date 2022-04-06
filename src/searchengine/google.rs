use super::*;
use reqwest::{Client, self};
use std::time::Duration;
use scraper::{Html, Selector};

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
        
        let search_selector = Selector::parse("rso").unwrap();
        let result_selector = Selector::parse(".g").unwrap();

        let search = document.select(&search_selector).next().unwrap();

        let results: Vec<SearchListing> = search.select(&result_selector).map(|x| { 

            let snippet_select = Selector::parse(r#".NJo7tc"#).unwrap();
            let link_select = Selector::parse(".yuRUbf").unwrap();
            let title_select = Selector::parse("h3").unwrap();
            let div_select = Selector::parse("div").unwrap();

            let snippet = x.select(&snippet_select).next().unwrap();
            let link = x.select(&link_select).next().unwrap();
            let title = x.select(&title_select).next().unwrap();

            SearchListing {
                title: title.text().next().unwrap().to_string(),
                url: link.first_child().unwrap().value().as_element().unwrap().attr("href").unwrap().to_string(),
                description: snippet.select(&div_select).next().unwrap().text().next().unwrap().to_string()
            }
        }).collect();

        let _ = send.send(results);
    });

    Ok(Search{engine: Engine::Google, results: recv.await.expect("Panic in duckduckgo html decode")})
    
    //Ok(result)
}
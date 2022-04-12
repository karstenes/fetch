use super::*;
use reqwest::{Client, self};
use std::time::Duration;
use tokio;
use scraper::{Html, Selector};
use html_escape;
use std::fs::File;
use std::io::Write;
use log::info;

pub async fn search(query: &str, timeout: Duration) -> Result<Option<Search>, Error> {

    let start = tokio::time::Instant::now();

    let result = Client::builder()
        .user_agent("User-Agent: Mozilla/5.0 (Windows NT 10.0; rv:91.0) Gecko/20100101 Firefox/91.0")
        .build()?
        .get("https://html.duckduckgo.com/html")
        //.body(query.to_string())
        .timeout(timeout)
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    if cfg!(debug_assertions) {
        let mut f = File::create("debug/ddg.html").unwrap();
        write!(f, "{}", result).unwrap();
    }

    if result.contains(r#"class="no-results""#) {
        return Ok(None);
    }

    let scrape  = tokio::time::Instant::now();

    let (send, recv) = tokio::sync::oneshot::channel();

    rayon::spawn(move || {
        let document = Html::parse_document(&result);       
        let links_selector = Selector::parse("#links").unwrap();
        let result_selector = Selector::parse(".result").unwrap();

        let links = document.select(&links_selector).next().unwrap();

        let results: Vec<SearchListing> = links.select(&result_selector).filter_map(|x| {

            //println!("{}", x.inner_html()); 
            let title_select = Selector::parse(".result__a").unwrap();
            let title = x.select(&title_select).next().unwrap();
            let snippet_select = Selector::parse(".result__snippet").unwrap();
            let snippet = match x.select(&snippet_select).next() {
                Some(x) => x,
                None => {
                    return None;
                }
            };
            Some(SearchListing {
                title: html_escape::decode_html_entities(&title.inner_html()).to_string(),
                url: title.value().attr("href").unwrap().to_string(),
                description: snippet.text().to_owned().map(|x|x.to_string()).collect::<String>(),
                sources: vec![Engine::DuckDuckGo],
                quality: 3,
            })
        }).collect();

        let _ = send.send(results);
    });
    info!("DDG request took {}, Scraping took {}", start.elapsed().as_secs_f32()-scrape.elapsed().as_secs_f32(), scrape.elapsed().as_secs_f32());
    Ok(Some(Search{engine: Engine::DuckDuckGo, results: recv.await.expect("Panic in duckduckgo html decode")}))
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    #[quickcheck_async::tokio]
    async fn searchtest(query: String) -> bool {
        let search = super::search(&query, Duration::new(5,0)).await;
        match search {
            Ok(_) => return true,
            Err(_) => return false
        }
    } 
}
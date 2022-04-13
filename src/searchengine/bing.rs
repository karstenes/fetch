use super::*;
use reqwest::{Client, self};
use std::time::Duration;
use tokio;
use scraper::{Html, Selector};
use html_escape;
use log::info;

use std::fs::File;
use std::io::Write;


const RESULTS_PATH: &str = "#b_results";
const RESULT_PATH: &str = "li.b_algo";
//const URL_PATH: &str = "h2>a";
const DESCRIPTION_NORMAL_PATH: &str = "div.b_caption>p";
const DESCRIPTION_CARD_PATH: &str = "li>div>span";
const TITLE_PATH: &str = "h2>a";

pub async fn search(query: &str, timeout: Duration) -> Result<Option<Search>, Error> {

    let start = tokio::time::Instant::now();

    let req = Client::builder()
        .user_agent("User-Agent: Mozilla/5.0 (Windows NT 10.0; rv:91.0) Gecko/20100101 Firefox/91.0")
        .build()?
        .get("https://www.bing.com/search")
        //.body(query.to_string())
        .timeout(timeout)
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?;
    
    if !req.url().as_str().starts_with("https://www.bing.com/search") {
        return Err(Error::RedirectError(Engine::Bing, req.url().as_str().to_string()));
    } 

    let result = req
        .text()
        .await?;

    if result.contains(r#"class="b_no""#) {
        return Ok(None);
    }

    if cfg!(feature = "savefile") {
        let mut f = File::create("debug/bing.html").unwrap();
        write!(f, "{}", result).unwrap();
    }

    let scrape  = tokio::time::Instant::now();

    let (send, recv) = tokio::sync::oneshot::channel();

    rayon::spawn(move || {
        let document = Html::parse_document(&result);
        
        let results_selector = Selector::parse(RESULTS_PATH).unwrap();

        let search = document.select(&results_selector).next().unwrap();

        let result_selector = Selector::parse(RESULT_PATH).unwrap();

        let results: Vec<SearchListing> = search.select(&result_selector).filter_map(|x| {
            //println!("{}", x.inner_html()); 
            let title_select = Selector::parse(TITLE_PATH).unwrap();
            let title = x.select(&title_select).next().unwrap();
            let snippet_select = Selector::parse(DESCRIPTION_NORMAL_PATH).unwrap();
            let snippet = match x.select(&snippet_select).next() {
                Some(x) => {x}
                None => {
                    let snippet_select = Selector::parse(DESCRIPTION_CARD_PATH).unwrap();
                    x.select(&snippet_select).next()?
                }
            };
            Some(SearchListing {
                title: html_escape::decode_html_entities(&title.inner_html()).to_string(),
                url: title.value().attr("href").unwrap().to_string(),
                description: snippet.text().to_owned().map(|x|x.to_string()).collect::<String>(),
                sources: vec![Engine::Bing],
                quality: 2,
            })
        }).collect();

        let _ = send.send(results);
    });
    info!("Bing request took {}, Scraping took {}", start.elapsed().as_secs_f32()-scrape.elapsed().as_secs_f32(), scrape.elapsed().as_secs_f32());
    Ok(Some(Search{engine: Engine::Bing, results: recv.await.expect("Panic in bing html decode")}))
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    #[quickcheck_async::tokio]
    async fn searchtest(query: String) -> bool {
        let search = search(&query, Duration::new(5,0)).await;
        match search {
            Ok(_) => return true,
            Err(e) => {
                println!("{:?}", e);
                match e {
                    Error::RedirectError(..) => return true,
                    Error::CaptchaError(_) => return true,
                    Error::ReqwestError(r) => {
                        if r.is_timeout() {
                            println!("Timeout");
                            return true;
                        } else if r.status().is_some() {
                            if r.status().unwrap().as_u16() == 403 {
                                return true;
                            }
                            return false;
                        } else {
                            println!("reqwest error");
                            return false;
                        }
                    }
                }
            }
        }
    } 
}
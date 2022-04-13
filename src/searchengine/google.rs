use super::*;
use reqwest::{Client, self};
use std::time::Duration;
use scraper::{Html, Selector};
use log::info;
use std::fs::File;
use std::io::Write;

const RESULTS_PATH: &str = "#rso";
const RESULT_PATH: &str = "div>.g";
const URL_PATH: &str = "div.yuRUbf>a";
const VID_URL_PATH: &str = "div.ct3b9e>a";
const DESCRIPTION_PATH: &str = "div.VwiC3b";
const VID_DESCRIPTION_PATH: &str = "div.Uroaid";
const TITLE_PATH: &str = "h3";

pub async fn search(query: &str, timeout: Duration) -> Result<Option<Search>, Error> {
    if query.is_empty() {
        info!("DDG search query was empty");
        return Ok(None)
    };

    let start = tokio::time::Instant::now();

    let request = Client::builder()
        .user_agent("User-Agent: Mozilla/5.0 (Windows NT 10.0; rv:91.0) Gecko/20100101 Firefox/91.0")
        .build()?
        .get("https://www.google.com/search")
        .timeout(timeout)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .header("Accept-Language", "en-US")
        .query(&[("q", query), ("ie", "utf8"), ("oe", "utf8"), ("hl", "en")])
        .send()
        .await?
        .error_for_status()?;

    if request.url().as_ref().contains("sorry.google.com") {
        return Err(Error::CaptchaError(Engine::Google))
    }

    if !request.url().as_str().starts_with("https://www.google.com/search") {
        return Err(Error::RedirectError(Engine::Google, request.url().as_str().to_string()));
    } 

    let result = request.text().await?;

    if cfg!(feature = "savefile") {
        let mut f = File::create("debug/google.html").unwrap();
        write!(f, "{}", result).unwrap();
    }

    let scrape  = tokio::time::Instant::now();
    let (send, recv) = tokio::sync::oneshot::channel();
    
    rayon::spawn(move || {
        let document = Html::parse_document(&result);

        let results_selector = Selector::parse(RESULTS_PATH).unwrap();


        let results = match document.select(&results_selector).next() {
            Some(o) => o,
            None => {
                send.send(None).unwrap();
                return;
            }
        };
    
        let result_selector = Selector::parse(RESULT_PATH).unwrap();
        
        let results: Vec<SearchListing> = results.select(&result_selector)
        .filter_map(|x| {
            let badres_select = Selector::parse("g-class-with-header").unwrap();
            match x.select(&badres_select).next() {
                Some(_) => {
                    return None;
                }
                None => {
                    ()
                }
            }
            //println!("{}\n", x.inner_html());
            let result_selector = Selector::parse(RESULT_PATH).unwrap();
            if let Some(_) = x.select(&result_selector).next() {
                return None
            }
            if let Some(o) = x.first_child().unwrap().value().as_element().unwrap().attr("class") {
                if o.contains("kp-wholepage") {
                    return None;
                }
            }
            let snippet_select = Selector::parse(DESCRIPTION_PATH).unwrap();
            let link_select = Selector::parse(URL_PATH).unwrap();
            let title_select = Selector::parse(TITLE_PATH).unwrap();

            

            let snippet = match x.select(&snippet_select).next() {
                Some(x) => x,
                None => {
                    let video_description_select = Selector::parse(VID_DESCRIPTION_PATH).unwrap();
                    x.select(&video_description_select).next()?
                }
            };
            let link = match x.select(&link_select).next() {
                Some(x) => x,
                None => {
                    let video_link_select = Selector::parse(VID_URL_PATH).unwrap();
                    x.select(&video_link_select).next().unwrap()
                }
            };
            let title = x.select(&title_select).next().unwrap();

            Some(SearchListing {
                title: title.text().to_owned().map(|x|x.to_string()).collect::<String>(),
                url: link.value().attr("href").unwrap().to_string(),
                description: snippet.text().to_owned().map(|x|x.to_string()).collect::<String>(),
                sources: vec![Engine::Google],
                quality: 2
            })
        }).collect();

        let _ = send.send(Some(results));
    });
    let x = recv.await.expect("Panic in google html decode");

    info!("Google request took {}, Scraping took {}", start.elapsed().as_secs_f32()-scrape.elapsed().as_secs_f32(), scrape.elapsed().as_secs_f32());
    if let None = x {
        return Ok(None);
    }
    Ok(Some(Search{engine: Engine::Google, results: x.unwrap()}))
    
    //Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    #[quickcheck_async::tokio]
    async fn searchtest(query: String) -> bool {
        let search = super::search(&query, Duration::new(5,0)).await;
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
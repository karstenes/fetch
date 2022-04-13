pub mod google;
pub mod duckduckgo;
pub mod bing;
pub mod brave;

use reqwest;
use scraper;

#[derive(Debug, Clone)]
pub struct CustomEngine {
    pub name: String,
    pub quality: u8,
    pub search_url: String,
    pub get: bool,
    pub search_query_key: String,
    pub results_container: String,
    pub result_container: String,
    pub title_selector: String,
    pub url_selector: String,
    pub description_selector: String,
    pub no_results_string: String,
}

#[derive(Debug, Clone)]
pub enum Engine {
    Google,
    DuckDuckGo,
    Bing,
    Brave,
    CustomEngine(CustomEngine),
}

#[derive(Debug, Clone)]
pub struct SearchListing {
    pub title: String,
    pub url: String,
    pub description: String,
    pub sources: Vec<Engine>,
    pub quality: u8,
}

#[derive(Debug, Clone)]
pub struct Search {
    pub results: Vec<SearchListing>,
    pub engine: Engine,

}

#[derive(Debug)]
pub enum Error {
    CaptchaError(Engine),
    RedirectError(Engine, String),
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::ReqwestError(error)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RedirectError(x, y) => {
                write!(f, "Got redirected by {:?} to {}", x, y)
            }
            Error::CaptchaError(x) => {
                write!(f, "Got captcha from {:?}", x)
            }
            Error::ReqwestError(x) => {
                write!(f, "{:}", x)
            }
        }
        
    }
}
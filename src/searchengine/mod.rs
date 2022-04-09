pub mod google;
pub mod duckduckgo;
pub mod bing;

use reqwest;
use scraper;

#[derive(Debug, Clone)]
pub enum Engine {
    Google,
    DuckDuckGo,
    Bing
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
            Error::CaptchaError(x) => {
                write!(f, "Got captcha from {:?}", x)
            }
            Error::ReqwestError(x) => {
                write!(f, "{:}", x)
            }
        }
        
    }
}
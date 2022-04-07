pub mod google;
pub mod duckduckgo;

use serde::Serialize;

#[derive(Debug, Clone)]
pub enum Engine {
    Google,
    DuckDuckGo,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchListing {
    pub title: String,
    pub url: String,
    pub description: String
}

#[derive(Debug, Clone)]

pub struct Search {
    pub results: Vec<SearchListing>,
    pub engine: Engine,

}
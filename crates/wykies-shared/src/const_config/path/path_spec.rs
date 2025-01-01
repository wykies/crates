use reqwest_cross::reqwest::{self, Method};

#[derive(Debug, Clone)]
pub struct PathSpec {
    pub path: &'static str,
    pub method: reqwest::Method,
}

impl PathSpec {
    pub const fn get(path: &'static str) -> Self {
        Self {
            path,
            method: Method::GET,
        }
    }

    pub const fn post(path: &'static str) -> Self {
        Self {
            path,
            method: Method::POST,
        }
    }
}

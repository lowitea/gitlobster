use reqwest::blocking as rqw;
use reqwest::Method;
use url::Url;

use crate::gitlab::types;

const OBJECTS_PER_PAGE: i32 = 100;
const API_VERSION: &str = "v4";

pub struct Client {
    url: Url,
    http: rqw::Client,
}

impl Client {
    pub fn new(token: String, url: String) -> Result<Self, String> {
        let http = rqw::Client::new();

        let mut url = Url::parse(&url).map_err(|e| e.to_string())?;
        let query = format!("access_token={}&per_page={}", token, OBJECTS_PER_PAGE);
        url.set_path(&format!("api/{}", API_VERSION));
        url.set_query(Some(&query));

        Ok(Client { url, http })
    }

    fn request<S: Into<String>>(&self, m: Method, path: S) -> reqwest::Result<rqw::Response> {
        let mut url = self.url.clone();
        url.set_path(&format!("{}/{}", url.path(), path.into()));

        self.http
            .request(m, url)
            .send()?
            .error_for_status()
    }

    pub fn get_projects(&self) -> reqwest::Result<Vec<types::Project>> {
        self.request(Method::GET, "projects")?
            .json::<Vec<types::Project>>()
    }
}

use reqwest::blocking as rqw;
use reqwest::Method;
use serde::Serialize;
use url::Url;

use crate::gitlab::types;

const OBJECTS_PER_PAGE: i32 = 100;
const API_VERSION: &str = "v4";

pub struct Client {
    url: Url,
    http: rqw::Client,
}

impl Client {
    pub fn new(token: String, mut url: Url) -> Result<Self, String> {
        let http = rqw::Client::new();

        // TODO: add iterate by pages
        let query = format!("access_token={}&per_page={}", token, OBJECTS_PER_PAGE);
        url.set_path(&format!("api/{}", API_VERSION));
        url.set_query(Some(&query));

        Ok(Client { url, http })
    }

    fn build_request<S: Into<String>>(&self, m: Method, path: S) -> rqw::RequestBuilder {
        let mut url = self.url.clone();
        url.set_path(&format!("{}/{}", url.path(), path.into()));
        self.http
            .request(m, url)
            .header("Content-Type", "application/json")
    }

    fn request<S: Into<String>>(&self, m: Method, path: S) -> reqwest::Result<rqw::Response> {
        self.build_request(m, path).send()?.error_for_status()
    }

    pub fn get_project(&self, path: String) -> reqwest::Result<types::Project> {
        let path = urlencoding::encode(&path);
        self.request(Method::GET, format!("projects/{}", path))?
            .json::<types::Project>()
    }

    // TODO: remove copy-paste
    pub fn project_exist(&self, path: String) -> reqwest::Result<Option<types::Project>> {
        match self.get_project(path) {
            Ok(p) => Ok(Some(p)),
            Err(e) => {
                // TODO: remove unwrap
                if e.status().unwrap() == reqwest::StatusCode::NOT_FOUND {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn get_projects(&self) -> reqwest::Result<Vec<types::Project>> {
        self.request(Method::GET, "projects")?
            .json::<Vec<types::Project>>()
    }

    pub fn make_project(
        &self,
        name: String,
        group_id: types::GroupId,
    ) -> reqwest::Result<types::Project> {
        #[derive(Serialize)]
        struct MakeProjectRequest {
            name: String,
            path: String,
            namespace_id: types::GroupId,
        }

        let path = name.clone();
        let namespace_id = group_id;

        self.build_request(Method::POST, "projects")
            .json(&MakeProjectRequest { name, path, namespace_id })
            .send()?
            .error_for_status()?
            .json::<types::Project>()
    }

    pub fn get_group(&self, path: String) -> reqwest::Result<types::Group> {
        let path = urlencoding::encode(&path);
        self.request(Method::GET, format!("groups/{}", path))?
            .json::<types::Group>()
    }

    pub fn group_exist(&self, path: String) -> reqwest::Result<Option<types::Group>> {
        match self.get_group(path) {
            Ok(g) => Ok(Some(g)),
            Err(e) => {
                // TODO: remove unwrap
                if e.status().unwrap() == reqwest::StatusCode::NOT_FOUND {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn make_subgroup(
        &self,
        name: String,
        parent_id: types::GroupId,
    ) -> reqwest::Result<types::Group> {
        #[derive(Serialize)]
        struct MakeGroupRequest {
            name: String,
            path: String,
            parent_id: types::GroupId,
        }

        let path = name.clone();

        self.build_request(Method::POST, "groups")
            .json(&MakeGroupRequest { name, path, parent_id })
            .send()?
            .error_for_status()?
            .json::<types::Group>()
    }

    pub fn make_project_with_namespace(
        &self,
        mut path: Vec<String>,
        root_group: &types::Group,
    ) -> reqwest::Result<types::Project> {
        let mut parent_id = root_group.id;

        // TODO: remove unwrap
        let project_name = path.pop().unwrap();

        let mut current_namespace = root_group.full_path.clone();

        for group_name in path {
            current_namespace = format!("{}/{}", current_namespace, group_name);
            let group = if let Some(group) = self.group_exist(current_namespace.clone())? {
                group
            } else {
                self.make_subgroup(group_name, parent_id)?
            };

            parent_id = group.id;
        };

        match self.project_exist(format!("{}/{}", current_namespace, project_name))? {
            Some(p) => Ok(p),
            None => self.make_project(project_name, parent_id)
        }
    }
}

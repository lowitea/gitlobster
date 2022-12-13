use crate::gitlab::types;
use anyhow::Result;
use chrono::Utc;
use reqwest::{Method, RequestBuilder, Response};
use serde::Serialize;
use url::Url;

const API_VERSION: &str = "v4";

pub struct Client {
    url: Url,
    http: reqwest::Client,
}

impl Client {
    pub fn new(token: &str, mut url: Url, opp: Option<u32>) -> Result<Self> {
        let http = reqwest::Client::new();
        let opp = if let Some(opp) = opp { opp } else { 1000 };

        let query = format!("access_token={}&per_page={}", token, opp);
        url.set_path(&format!("api/{}", API_VERSION));
        url.set_query(Some(&query));

        Ok(Client { url, http })
    }

    fn build_request<S: Into<String>>(&self, m: Method, path: S) -> RequestBuilder {
        let mut url = self.url.clone();
        url.set_path(&format!("{}/{}", url.path(), path.into()));
        self.http
            .request(m, url)
            .header("Content-Type", "application/json")
    }

    async fn request<S: Into<String>>(&self, m: Method, path: S) -> reqwest::Result<Response> {
        self.build_request(m, path).send().await?.error_for_status()
    }

    pub async fn get_project(&self, path: String) -> reqwest::Result<types::Project> {
        let path = urlencoding::encode(&path);
        self.request(Method::GET, format!("projects/{}", path))
            .await?
            .json::<types::Project>()
            .await
    }

    fn exist<T>(&self, resp: reqwest::Result<T>) -> reqwest::Result<Option<T>> {
        match resp {
            Ok(p) => Ok(Some(p)),
            Err(e) => {
                if let Some(status) = e.status() {
                    if status == reqwest::StatusCode::NOT_FOUND {
                        return Ok(None)
                    }
                }
                Err(e)
            }
        }
    }

    pub async fn project_exist(&self, path: String) -> reqwest::Result<Option<types::Project>> {
        self.exist(self.get_project(path).await)
    }

    pub async fn get_projects(
        &self,
        only_owned: bool,
        only_membership: bool,
    ) -> Result<Vec<types::Project>> {
        let mut projects: Vec<types::Project> = vec![];
        let mut next_page = 1;

        loop {
            let mut url = self.url.clone();
            url.set_path(&format!("{}/{}", url.path(), "projects"));

            let mut query = url.query().expect("query is empty").to_string();
            query += format!("&page={}", next_page).as_str();
            if only_owned {
                query += "&owned=true"
            }
            if only_membership {
                query += "&only_membership=true"
            }
            url.set_query(Some(&query));

            let resp = self
                .http
                .request(Method::GET, url)
                .header("Content-Type", "application/json")
                .send()
                .await?
                .error_for_status()?;

            let headers = resp.headers().clone();

            projects.append(&mut resp.json::<Vec<types::Project>>().await?);

            let next_page_header = headers.get("x-next-page").unwrap();
            if next_page_header.is_empty() {
                break;
            }

            next_page += 1;
        }

        Ok(projects)
    }

    fn make_project_description(new_description: Option<String>) -> String {
        format!(
            "{} 🦞 Synced: {}",
            new_description.unwrap_or_default(),
            Utc::now().to_rfc3339()
        )
    }

    pub async fn make_project(
        &self,
        name: String,
        group_id: types::GroupId,
        info: &types::Project,
    ) -> reqwest::Result<types::Project> {
        #[derive(Serialize)]
        struct MakeProjectRequest {
            name: String,
            description: String,
            path: String,
            namespace_id: types::GroupId,
        }

        let path = name.clone();
        let namespace_id = group_id;
        let description = Client::make_project_description(info.description.clone());

        self.build_request(Method::POST, "projects")
            .json(&MakeProjectRequest {
                name,
                description,
                path,
                namespace_id,
            })
            .send()
            .await?
            .error_for_status()?
            .json::<types::Project>()
            .await
    }

    pub async fn update_project(
        &self,
        project: &types::Project,
        info: &types::Project,
    ) -> reqwest::Result<types::Project> {
        #[derive(Serialize)]
        struct UpdateProjectRequest {
            description: String,
        }

        let description = Client::make_project_description(info.description.clone());

        self.build_request(Method::PUT, format!("projects/{}", project.id))
            .json(&UpdateProjectRequest { description })
            .send()
            .await?
            .error_for_status()?
            .json::<types::Project>()
            .await
    }

    pub async fn get_group(&self, path: String) -> reqwest::Result<types::Group> {
        let path = urlencoding::encode(&path);
        self.request(Method::GET, format!("groups/{}", path))
            .await?
            .json::<types::Group>()
            .await
    }

    pub async fn group_exist(&self, path: String) -> reqwest::Result<Option<types::Group>> {
        self.exist(self.get_group(path).await)
    }

    pub async fn make_subgroup(
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
            .json(&MakeGroupRequest {
                name,
                path,
                parent_id,
            })
            .send()
            .await?
            .error_for_status()?
            .json::<types::Group>()
            .await
    }

    pub async fn make_project_with_namespace(
        &self,
        mut path: Vec<String>,
        root_group: &types::Group,
        project_info: &types::Project,
    ) -> reqwest::Result<types::Project> {
        let mut parent_id = root_group.id;

        // TODO: remove unwrap
        let project_name = path.pop().unwrap();

        let mut current_namespace = root_group.full_path.clone();

        for group_name in path {
            current_namespace = format!("{}/{}", current_namespace, group_name);
            let group = if let Some(group) = self.group_exist(current_namespace.clone()).await? {
                group
            } else {
                self.make_subgroup(group_name, parent_id).await?
            };

            parent_id = group.id;
        }

        match self
            .project_exist(format!("{}/{}", current_namespace, project_name))
            .await?
        {
            Some(p) => self.update_project(&p, project_info).await,
            None => {
                self.make_project(project_name, parent_id, project_info)
                    .await
            }
        }
    }

    pub async fn get_current_user(&self) -> reqwest::Result<types::User> {
        self.request(Method::GET, "user")
            .await?
            .json::<types::User>()
            .await
    }
}

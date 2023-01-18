use crate::gitlab::types;
use anyhow::Result;
use chrono::Utc;
use reqwest::{Method, RequestBuilder, Response};
use serde::Serialize;
use tracing::info;
use url::Url;

const API_VERSION: &str = "v4";

pub struct Client {
    url: Url,
    http: reqwest::Client,
    disable_sync_date: bool,
    token: String,
    limit: u32,
}

impl Client {
    pub fn new(
        token: &str,
        mut url: Url,
        opp: Option<u32>,
        disable_sync_date: bool,
    ) -> Result<Self> {
        let http = reqwest::Client::new();
        let limit = if let Some(opp) = opp { opp } else { 1000 };
        let token = token.to_string();

        url.set_path(&format!("api/{}", API_VERSION));

        Ok(Client {
            url,
            http,
            disable_sync_date,
            token,
            limit,
        })
    }

    fn build_request<S: Into<String>, J: Serialize>(
        &self,
        m: Method,
        path: S,
        query: Option<String>,
        json: Option<J>,
    ) -> RequestBuilder {
        let mut url = self.url.clone();
        url.set_path(&format!("{}/{}", url.path(), path.into()));

        if let Some(query) = query {
            url.set_query(Some(&query));
        }

        info!("{}", url);

        let mut req = self
            .http
            .request(m, url)
            .header("Content-Type", "application/json")
            .header("PRIVATE-TOKEN", &self.token);

        if let Some(json) = json {
            req = req.json(&json)
        }

        req
    }

    async fn request<S: Into<String>, J: Serialize>(
        &self,
        m: Method,
        path: S,
        query: Option<String>,
        json: Option<J>,
    ) -> reqwest::Result<Response> {
        self.build_request(m, path, query, json)
            .send()
            .await?
            .error_for_status()
    }

    pub async fn get_project(&self, path: String) -> reqwest::Result<types::Project> {
        let path = urlencoding::encode(&path);
        self.request(Method::GET, format!("projects/{}", path), None, None::<()>)
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
                        return Ok(None);
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
            let mut query = format!("per_page={}&page={}", &self.limit, next_page);
            if only_owned {
                query += "&owned=true"
            }
            if only_membership {
                query += "&only_membership=true"
            }
            let resp = self
                .request(Method::GET, "projects", Some(query), None::<()>)
                .await?;
            let headers = resp.headers().clone();

            projects.append(&mut resp.json::<Vec<types::Project>>().await?);

            let next_page_header = headers.get("x-next-page").unwrap();
            if next_page_header.is_empty() {
                break;
            }

            next_page += 1;
        }

        projects.retain(|p| !p.empty_repo);

        Ok(projects)
    }

    fn make_project_description(&self, new_description: Option<String>) -> String {
        if self.disable_sync_date {
            new_description.unwrap_or_default()
        } else {
            format!(
                "{} 🦞 Synced: {}",
                new_description.unwrap_or_default(),
                Utc::now().to_rfc3339()
            )
        }
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
        let description = self.make_project_description(info.description.clone());

        let data = &MakeProjectRequest {
            name,
            description,
            path,
            namespace_id,
        };
        self.request(Method::POST, "projects", None, Some(data))
            .await?
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

        let description = self.make_project_description(info.description.clone());
        let data = &UpdateProjectRequest { description };

        self.request(
            Method::PUT,
            format!("projects/{}", project.id),
            None,
            Some(data),
        )
        .await?
        .json::<types::Project>()
        .await
    }

    pub async fn get_group(&self, path: String) -> reqwest::Result<types::Group> {
        let path = urlencoding::encode(&path);
        self.request(Method::GET, format!("groups/{}", path), None, None::<()>)
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
        parent_id: Option<types::GroupId>,
    ) -> reqwest::Result<types::Group> {
        #[derive(Serialize)]
        struct MakeGroupRequest {
            name: String,
            path: String,
            parent_id: Option<types::GroupId>,
        }

        let path = name.clone();
        let data = &MakeGroupRequest {
            name,
            path,
            parent_id,
        };

        self.request(Method::POST, "groups", None, Some(data))
            .await?
            .json::<types::Group>()
            .await
    }

    pub async fn make_project_with_namespace(
        &self,
        mut path: Vec<String>,
        root_group: &Option<types::Group>,
        project_info: &types::Project,
    ) -> reqwest::Result<types::Project> {
        let mut parent_id = root_group.as_ref().map(|gr| gr.id);
        let project_name = path.pop().expect("invalid project path");
        let mut current_namespace = root_group
            .as_ref()
            .map(|gr| gr.full_path.clone())
            .unwrap_or_default();

        for group_name in path {
            current_namespace = if current_namespace.is_empty() {
                group_name.clone()
            } else {
                format!("{}/{}", current_namespace, group_name)
            };
            let group = if let Some(group) = self.group_exist(current_namespace.clone()).await? {
                group
            } else {
                self.make_subgroup(group_name, parent_id).await?
            };

            parent_id = Some(group.id);
        }

        match self
            .project_exist(format!("{}/{}", current_namespace, project_name))
            .await?
        {
            Some(p) => self.update_project(&p, project_info).await,
            None => {
                self.make_project(
                    project_name,
                    parent_id.unwrap_or_else(|| {
                        panic!(
                            "Parent group for project {} not found",
                            &project_info.name_with_namespace
                        )
                    }),
                    project_info,
                )
                .await
            }
        }
    }

    pub async fn get_current_user(&self) -> reqwest::Result<types::User> {
        self.request(Method::GET, "user", None, None::<()>)
            .await?
            .json::<types::User>()
            .await
    }
}

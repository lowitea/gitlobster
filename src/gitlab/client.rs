use anyhow::Result;
use chrono::Utc;
use reqwest::{header::HeaderValue, Method, RequestBuilder, Response};
use serde::Serialize;
use std::time::Duration;
use tracing::info;
use url::Url;

use crate::gitlab::types;

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
        timeout: Option<u32>,
    ) -> Result<Self> {
        let mut http = reqwest::ClientBuilder::new();
        if let Some(timeout) = timeout {
            http = http.timeout(Duration::from_secs(timeout.into()));
        }
        let http = http.build()?;
        let limit = if let Some(opp) = opp { opp } else { 100 };
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

    /// Parse value of Link header
    /// ```
    /// let link = r#"<https://gitlab.example.com/api/v4/projects?per_page=50&order_by=id&sort=asc; rel="next""#;
    /// let result = Client::parse_link_header(link);
    /// assert_eq!(
    ///     result,
    ///     "https://gitlab.example.com/api/v4/projects?per_page=50&order_by=id&sort=asc"
    /// )
    /// ```
    fn parse_link_header(link: &HeaderValue, next_page_link_position: usize) -> String {
        let invalid_link_msg = format!("invalid Link header ({:#?})", &link);
        let link = link
            .to_str()
            .expect(&invalid_link_msg)
            .split(';')
            .nth(next_page_link_position)
            .expect(&invalid_link_msg);
        if link.len() < 13 {
            panic!("{}", invalid_link_msg);
        }
        link[1..link.len() - 1].to_string()
    }

    pub async fn get_projects(
        &self,
        group: Option<String>,
        only_owned: bool,
        only_membership: bool,
    ) -> Result<Vec<types::Project>> {
        let mut projects: Vec<types::Project> = vec![];
        let mut next_page: Option<String> = None;

        let method = match group {
            None => "projects".to_owned(),
            Some(group) => format!("groups/{}/projects", group),
        };

        let mut next_page_link_position = 0;

        loop {
            let query = if let Some(next_page) = next_page {
                next_page
                    .split('?')
                    .last()
                    .unwrap_or_else(|| {
                        panic!(
                            "Invalid url returned in Link header from GitLab ({})",
                            &next_page
                        )
                    })
                    .to_string()
            } else {
                let mut query = format!("order_by=id&sort=asc&per_page={}", &self.limit);
                if only_owned {
                    query += "&owned=true"
                }
                if only_membership {
                    query += "&only_membership=true"
                }
                if method != "projects" {
                    query += "&include_subgroups=true"
                }
                query
            };

            let resp = self
                .request(Method::GET, &method, Some(query), None::<()>)
                .await?;

            let headers = resp.headers().clone();

            match headers.get("x-next-page") {
                None => break,
                Some(has_next_page) => {
                    if has_next_page
                        .to_str()
                        .expect("Invalid x-next-page header")
                        .is_empty()
                    {
                        break;
                    }
                }
            }

            let Some(link_header) = headers.get("link") else {
                break;
            };

            next_page = Some(Client::parse_link_header(
                link_header,
                next_page_link_position,
            ));

            projects.append(&mut resp.json::<Vec<types::Project>>().await?);
            next_page_link_position = 1;
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
        slug: String,
        group_id: u32,
        info: &types::Project,
    ) -> reqwest::Result<types::Project> {
        #[derive(Serialize)]
        struct MakeProjectRequest {
            name: String,
            description: String,
            path: String,
            namespace_id: u32,
        }

        let name = info.name.clone();
        let path = slug.clone();
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

    pub async fn get_group(&self, path: &str) -> reqwest::Result<types::Group> {
        let path = urlencoding::encode(path);
        self.request(Method::GET, format!("groups/{}", path), None, None::<()>)
            .await?
            .json::<types::Group>()
            .await
    }

    pub async fn group_exist(&self, path: &str) -> reqwest::Result<Option<types::Group>> {
        self.exist(self.get_group(path).await)
    }

    pub async fn make_subgroup(
        &self,
        name: String,
        path: String,
        parent_id: Option<u32>,
    ) -> reqwest::Result<types::Group> {
        #[derive(Serialize)]
        struct MakeGroupRequest {
            name: String,
            path: String,
            parent_id: Option<u32>,
        }

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
        groups: Vec<types::Group>,
        root_group: &Option<types::Group>,
        project_info: &types::Project,
    ) -> reqwest::Result<types::Project> {
        let mut parent_id = root_group.as_ref().map(|gr| gr.id);
        let project_slug = path.pop().expect("invalid project path");
        let mut current_namespace = root_group
            .as_ref()
            .map(|gr| gr.full_path.clone())
            .unwrap_or_default();

        for group in groups {
            current_namespace = if current_namespace.is_empty() {
                group.name.clone()
            } else {
                format!("{}/{}", current_namespace, &group.path)
            };
            let group = if let Some(group) = self.group_exist(&current_namespace).await? {
                group
            } else {
                self.make_subgroup(group.name.clone(), group.path.clone(), parent_id)
                    .await?
            };

            parent_id = Some(group.id);
        }

        match self
            .project_exist(format!("{}/{}", current_namespace, project_slug))
            .await?
        {
            Some(p) => self.update_project(&p, project_info).await,
            None => {
                self.make_project(
                    project_slug,
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

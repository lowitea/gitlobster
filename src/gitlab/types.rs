use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub name: String,
    pub id: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub id: u32,
    pub description: Option<String>,
    /// Whether the project has an empty repository or not.
    pub empty_repo: bool,
    /// The URL to clone the repository over SSH.
    pub ssh_url_to_repo: String,
    /// The URL to clone the repository over HTTPS.
    pub http_url_to_repo: String,
    /// The display name of the project.
    pub name: String,
    /// The display name of the project with the namespace.
    pub name_with_namespace: String,
    /// The path to the project's repository.
    pub path: String,
    /// The path to the project's repository with its namespace.
    pub path_with_namespace: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Group {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub full_path: String,
}

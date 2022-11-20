use reqwest::blocking as rqw;
use std::fs::{self, File};
use std::io::Read;
use subprocess::{Exec, ExitStatus};
use uuid::Uuid;

const OUT_DIR: &str = "tests/test_out";
const GITLAB_HOST: &str = "https://gitlab.com/";

fn check_local(updated_data: Option<&String>) {
    println!("Check local dir");
    fn check_file_data(path: String, data: &str) {
        let mut file = File::open(path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        let content = content.trim();
        println!("content: {}, expected: {}", content, data);
        assert!(content.trim() == data);
    }

    let prefix = format!("{}/gitlobster_test/download", OUT_DIR);
    let p1_path = format!("{}/project_1", prefix);
    let p1_file_path = format!("{}/project_1", p1_path);
    let files = vec![
        (p1_file_path.clone(), "branch1"),
        (format!("{}/project_2/project_2", prefix), "main"),
        (format!("{}/subgroup_1/project_3/project_3", prefix), "main"),
    ];

    // check files in default branches
    for (path, data) in files {
        check_file_data(path, data);
    }

    // check the second branch in project_1
    let git_cmd = format!("git -C {} checkout branch2", p1_path);
    Exec::shell(git_cmd).join().unwrap();
    check_file_data(p1_file_path, "branch2");

    // check updated file if need
    if let Some(data) = updated_data {
        check_file_data(format!("{}/updating", p1_path), data.as_str());
    };
    let git_cmd = format!("git -C {} checkout branch1", p1_path);
    Exec::shell(git_cmd).join().unwrap();
}

fn check_backup(gitlab_token: &str, updated_data: Option<&String>) {
    println!("Check backup");
    let prefix = "gitlobster_test%2Fupload%2Fgitlobster_test%2Fdownload";
    let mut files = vec![
        (
            format!("{}%2Fproject_1", prefix),
            "branch1",
            "project_1",
            "branch1",
        ),
        (
            format!("{}%2Fproject_1", prefix),
            "branch2",
            "project_1",
            "branch2",
        ),
        (
            format!("{}%2Fproject_2", prefix),
            "main",
            "project_2",
            "main",
        ),
        (
            format!("{}%2Fsubgroup_1%2Fproject_3", prefix),
            "main",
            "project_3",
            "main",
        ),
    ];

    // check updated file if need
    if let Some(data) = updated_data {
        files.push((
            format!("{}%2Fproject_1", prefix),
            "branch2",
            "updating",
            data,
        ));
    };

    let client = rqw::Client::new();
    for (project, branch, file, data) in files {
        let url = format!(
            "{}api/v4/projects/{}/repository/files/{}/raw?ref={}&access_token={}",
            GITLAB_HOST, project, file, branch, gitlab_token
        );
        let resp = client.get(url).send().unwrap().error_for_status().unwrap();
        let content = resp.text().unwrap();
        let content = content.trim();
        println!("content: {}, expected: {}", content, data);
        assert!(content == data);
    }
}

fn cleanup(gitlab_token: &str) {
    println!("Cleanup test objects");

    let _ = fs::remove_dir_all(OUT_DIR);
    let url = format!(
        "{}api/v4/groups/gitlobster_test%2Fupload%2Fgitlobster_test?access_token={}",
        GITLAB_HOST, gitlab_token
    );
    let _ = rqw::Client::new().delete(url).send();
}

fn run_cmd(gitlab_token: &str) -> ExitStatus {
    Exec::shell(format!(
        "cargo run -- \
        --ft={} \
        --fu={} \
        --bt={} \
        --bu={} \
        --bg=gitlobster_test/upload \
        --only-owned \
        --include='^gitlobster_test/download' \
        --concurrency-limit=1 \
        -vv \
        {}",
        gitlab_token, GITLAB_HOST, gitlab_token, GITLAB_HOST, OUT_DIR,
    ))
    .join()
    .unwrap()
}

fn update_remote_project(gitlab_token: &str) -> String {
    let project = "gitlobster_test%2Fdownload%2Fproject_1";
    let file = "updating";
    let url = format!(
        "{}api/v4/projects/{}/repository/files/{}?access_token={}",
        GITLAB_HOST, project, file, gitlab_token
    );
    let id = Uuid::new_v4().to_string();
    let data = format!(
        r#"{{
        "branch": "branch2",
        "author_email": "me@gitlobster.sea",
        "author_name": "Mr. Gitlobster",
        "content": "{}",
        "commit_message": "update"
    }}"#,
        id
    );
    rqw::Client::new()
        .put(url)
        .body(data)
        .header("Content-Type", "application/json")
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    id
}

#[test]
fn test_general() {
    let gitlab_token = match option_env!("GTLBSTR_TEST_GITLAB_TOKEN") {
        Some(token) => token,
        None => return,
    };

    cleanup(gitlab_token);

    let exit_status = run_cmd(gitlab_token);
    assert!(exit_status.success());

    check_local(None);
    check_backup(gitlab_token, None);

    println!("check updating project");
    let expected = update_remote_project(gitlab_token);

    let exit_status = run_cmd(gitlab_token);
    assert!(exit_status.success());

    check_local(Some(&expected));
    check_backup(gitlab_token, Some(&expected));

    cleanup(gitlab_token);
}

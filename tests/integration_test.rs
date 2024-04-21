#[cfg(feature = "integration_tests")]
mod tests {
    use chrono::{DateTime, Utc};
    use reqwest::blocking as rqw;
    use serde::Deserialize;
    use std::fs::{self, File};
    use std::io::Read;
    use std::vec;
    use subprocess::{Exec, ExitStatus};
    use uuid::Uuid;

    const OUT_DIR: &str = "/tmp/gitlobster/tests/test_out";
    const GITLAB_HOST: &str = "https://gitlab.com/";

    fn check_local(updated_data: Option<&String>) {
        println!("-- check local dir");
        fn check_file_data(path: String, data: &str) {
            let mut file = File::open(path).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();

            let content = content.trim();
            println!("-- content: {}, expected: {}", content, data);
            assert!(content.trim() == data);
        }

        let prefix = format!("{}/gitlobster_test/example", OUT_DIR);
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

    fn check_backup(start_time: DateTime<Utc>, gitlab_token: &str, updated_data: Option<&String>) {
        println!("-- check backup");

        #[derive(Deserialize)]
        struct Project {
            description: String,
            name: String,
        }

        let url_prefix = format!("{}api/v4/projects", GITLAB_HOST);
        let project_path = "gitlobster_test%2Fupload3%2Fgitlobster_test%2Fexample";
        let p1_name = format!("{}%2Fproject_1", project_path);
        let p2_name = format!("{}%2Fproject_2", project_path);
        let p3_name = format!("{}%2Fsubgroup_1%2Fproject_3", project_path);
        let mut files = vec![
            (&p1_name, "branch1", "project_1", "branch1"),
            (&p1_name, "branch2", "project_1", "branch2"),
            (&p2_name, "main", "project_2", "main"),
            (&p3_name, "main", "project_3", "main"),
        ];

        // check updated file if need
        if let Some(data) = updated_data {
            files.push((&p1_name, "branch2", "updating", data));
        };

        let client = rqw::Client::new();
        for (project, branch, file, data) in files {
            let url = format!(
                "{}/{}/repository/files/{}/raw?ref={}&access_token={}",
                url_prefix, project, file, branch, gitlab_token
            );
            let resp = client.get(url).send().unwrap().error_for_status().unwrap();
            let content = resp.text().unwrap();
            let content = content.trim();
            println!("-- content: {}, expected: {}", content, data);
            assert!(content == data);
        }

        // check description
        let projects = vec![
            (p1_name, "project_1"),
            (p2_name, "Project 2"),
            (p3_name, "project_3"),
        ];
        for (project, project_name) in projects {
            let url = format!("{}/{}?access_token={}", url_prefix, project, gitlab_token);
            let resp = client.get(url).send().unwrap().error_for_status().unwrap();
            let p = resp.json::<Project>().unwrap();
            let d_time_str = p.description.split(" 🦞 Synced: ").last().unwrap();
            let d_time = DateTime::parse_from_rfc3339(d_time_str).unwrap();
            assert!(d_time >= start_time);
            assert!(p.name == project_name);
        }
    }

    fn cleanup(gitlab_token: &str) {
        println!("-- cleanup test objects");

        let _ = fs::remove_dir_all(OUT_DIR);
        let url = format!(
            "{}api/v4/groups/gitlobster_test%2Fupload3%2Fgitlobster_test?access_token={}",
            GITLAB_HOST, gitlab_token
        );
        let _ = rqw::Client::new().delete(url).send();
    }

    fn run_gitlobster(gitlab_token: &str, enable_ssh: bool) -> ExitStatus {
        let mut cmd = format!(
            "cargo run -- \
            --ft={} \
            --fu={} \
            --bt={} \
            --bu={} \
            --bg=gitlobster_test/upload3 \
            --only-owned \
            --include='^gitlobster_test/example' \
            --concurrency-limit=1 \
            -d {} \
            -vv",
            gitlab_token, GITLAB_HOST, gitlab_token, GITLAB_HOST, OUT_DIR,
        );
        if enable_ssh {
            cmd = format!("{} --download-ssh --upload-ssh", cmd)
        }
        Exec::shell(cmd).join().unwrap()
    }

    fn update_remote_project(gitlab_token: &str) -> String {
        let project = "gitlobster_test%2Fexample%2Fproject_1";
        let file = "updating";
        let url = format!(
            "{}api/v4/projects/{}/repository/files/{}?access_token={}",
            GITLAB_HOST, project, file, gitlab_token
        );
        let id = Uuid::new_v4().to_string();
        let data = format!(
            r#"{{
                "branch": "branch2",
                "author_email": "gitlobster@lowit.ru",
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
        let gitlab_token = option_env!("GTLBSTR_TEST_GITLAB_TOKEN");
        let gitlab_token = gitlab_token.expect("required GTLBSTR_TEST_GITLAB_TOKEN env");

        cleanup(gitlab_token);

        println!("-- check first run");

        let start_time = Utc::now();
        let exit_status = run_gitlobster(gitlab_token, false);
        assert!(exit_status.success());

        check_local(None);
        check_backup(start_time, gitlab_token, None);

        println!("-- check updating project");
        let expected = update_remote_project(gitlab_token);

        let start_time = Utc::now();
        let exit_status = run_gitlobster(gitlab_token, false);
        assert!(exit_status.success());

        check_local(Some(&expected));
        check_backup(start_time, gitlab_token, Some(&expected));

        cleanup(gitlab_token);

        println!("-- check cloning by ssh");

        let start_time = Utc::now();
        let exit_status = run_gitlobster(gitlab_token, true);
        assert!(exit_status.success());

        check_local(None);
        check_backup(start_time, gitlab_token, None);

        cleanup(gitlab_token);
    }
}

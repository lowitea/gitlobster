# Gitlobster

**_A tool for full cloning all available repositories from a GitLab server._**

[![Crates.io](https://img.shields.io/crates/v/gitlobster?style=for-the-badge)](https://crates.io/crates/gitlobster)
[![Docker Image Version (latest semver)](https://img.shields.io/docker/v/lowitea/gitlobster?sort=semver&label=docker&style=for-the-badge)](https://hub.docker.com/r/lowitea/gitlobster)
[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/lowitea/gitlobster/Integration%20tests?label=integration%20tests&style=for-the-badge)](https://github.com/lowitea/gitlobster/actions)
[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/lowitea/gitlobster/Tests?label=unit%20tests&style=for-the-badge)](https://github.com/lowitea/gitlobster/actions)

EN | [RU](README.RU.md)

<br>
<br>

<p align="center"><img src="https://github.com/lowitea/gitlobster/raw/master/logo.png"></p>

<br>

## Key features

- Clone all available repositories
- Clone all branches from each repository
- Upload all repositories to another GitLab server (or a group in the same GitLab)
- Download only updates (including all newly added repositories) after the first full cloning
- Preserve the group hierarchy
- Support filters (include regexp templates) for cloning only necessary repository

## Install

### Docker

```shell
docker run --rm -it lowitea/gitlobster:latest --help
```

### Running pre-assembled binary files

1. Download an archive from [the release page](https://github.com/lowitea/gitlobster/releases) for your OS.
2. Unpack the archive.
3. Run the `gitlobster` file.

### Cargo

```shell
cargo install gitlobster
```

### Building from source

```shell
# clone the repository
git clone https://github.com/lowitea/gitlobster.git
# going to the downloaded directory
cd gitlobster
# build
cargo build --release
# run
./target/release/gitlobster --help
```

_The option to run it in the developer mode without pre-build is also available._

```shell
# in the project directory
cargo run -- --help
```

## Usage

### GitLab Token

In order for the tool to work, you need to generate a GitLab token with API read rights (`read_api`). If SSH copying is not used, then you will also a need permission to read repositories (`read_repository`).

If a second GitLab is used to copy repositories there, then a token is also required for it. Full API rights are required (`api`). If SSH upload is not used, then you will also need write permissions for repositories (`write_repository`).

You can generate tokens on [the settings page](https://github.com/-/profile/personal_access_tokens).

### SSH

If SSH copying is used, then ssh keys must be [added](https://gitlab.com/-/profile/keys) in GitLab.

### Help command

```text
$ gitlobster --help

A tool for cloning all available repositories in a GitLab instance

Usage: gitlobster [OPTIONS] --fu <FETCH URL> --ft <FETCH TOKEN>

Options:
      --fu <FETCH URL>             The GitLab instance URL for fetch repositories (example: https://gitlab.local/) [env: GTLBSTR_FETCH_URL=]
      --ft <FETCH TOKEN>           Your personal GitLab token for fetch repositories [env: GTLBSTR_FETCH_TOKEN=]
      --bu <BACKUP URL>            The GitLab instance URL for backup repositories (example: https://backup-gitlab.local/) [env: GTLBSTR_BACKUP_URL=]
      --bt <BACKUP TOKEN>          Your personal GitLab token for backup repositories [env: GTLBSTR_BACKUP_TOKEN=]
      --bg <BACKUP GROUP>          A target created group on backup GitLab for push repositories [env: GTLBSTR_BACKUP_GROUP=]
      --include <PATTERN>          Include regexp patterns (cannot be used together with --exclude flag, may be repeated) [env: GTLBSTR_INCLUDE=]
      --exclude <PATTERN>          Comma separated exclude regexp patterns (cannot be used together with --include flag, may be repeated) [env: GTLBSTR_EXCLUDE=]
  -d, --dst <DIRECTORY>            A destination local folder for save downloaded repositories [env: GTLBSTR_DST=]
  -v, --verbose...                 Verbose level (one or more, max four)
      --dry-run                    Show all projects to download
      --objects-per-page <COUNT>   Low-level option, how many projects can fetch in one request [env: GTLBSTR_OBJECTS_PER_PAGE=]
      --limit <COUNT>              Maximum projects to download [env: GTLBSTR_LIMIT=]
      --concurrency-limit <LIMIT>  Limit concurrency download [env: GTLBSTR_CONCURRENCY_LIMIT=] [default: 21]
      --only-owned                 Download projects explicitly owned by user [env: GTLBSTR_ONLY_OWNED=]
      --only-membership            Download only user's projects [env: GTLBSTR_ONLY_MEMBERSHIP=]
      --download-ssh               Enable download by ssh instead of http. An authorized ssh key is required [env: GTLBSTR_DOWNLOAD_SSH=]
      --upload-ssh                 Enable upload by ssh instead of http. An authorized ssh key is required [env: GTLBSTR_UPLOAD_SSH=]
      --disable-hierarchy          Disable saving the directory hierarchy [env: GTLBSTR_DISABLE_HIERARCHY=]
      --clear-dst                  Clear dst path before cloning [env: GTLBSTR_CLEAR_DST=]
  -h, --help                       Print help information
  -V, --version                    Print version information
```

### Copying all repositories to a second GitLab

```shell
gitlobster --ft=<FETCH_TOKEN> --fu=https://gitlab.com/ --bt=<UPLOAD_TOKEN> --bu=https://gitlab.com/ --bg=gitlobster_test/upload
```

### Download all repositories to a local directory

```shell
gitlobster --ft=<FETCH_TOKEN> --fu=https://gitlab.com/ -d out_directory
```

_Simultaneous saving repositories to a local directory and a second GitLab is supported._

### Using filters and filtering flags

```shell
gitlobster --ft=<FETCH_TOKEN> --fu=https://gitlab.com/ --only-owned --include="^gitlobster_test/download/project_2" --include="^gitlobster_test/download/project_1" -d out_directory
```

_It's also possible to use `--exclude` flag to load all repositories except repositories matching a necessary template._

_Simultaneous use of both `--exclude` and `--include` flags isn't allowed._

## Analogues

- [gitlabber](https://github.com/ezbz/gitlabber)
- [ghorg](https://github.com/gabrie30/ghorg)

# Gitlobster

___A tool for cloning all available repositories in a GitLab instance.___

Inspired by [gitlabber](https://github.com/ezbz/gitlabber).

## Features

- Cloning all available repositories from a Gitlab instance to a local folder while preserving the directory tree
- Clone all branches
- Update all cloned repositories
- Clone new repositories that have appeared
- Push the cloned repositories to a new GitLab instance, keeping the directory tree

## Install (TDB)

[//]: # (TODO: Write a complete installation guide)

```shell
cargo build --release
```

## Usage

```shell
gitlobster -t token -u url DESTINATION_DIR
```

## TODO

- [ ] Clone all available repositories from a Gitlab instance to a group on another GitLab instance while preserving the
  directory structure
- [ ] Save not only the directory tree but also repository settings
- [ ] Add cloning by http
- [ ] Add include/exclude patterns

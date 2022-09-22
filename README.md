# Gitlobster

___A tool for cloning all available repositories in a GitLab instance.___

Inspired by [gitlabber](https://github.com/ezbz/gitlabber).

<br>
<br>
<br>

<p align="center">
    <img src=" https://github.com/lowitea/gitlobster/raw/master/logo.svg">
</p>

## Features

- Cloning all available repositories from a Gitlab instance to a local folder while preserving the directory tree
- Clone all branches
- Update all cloned repositories
- Clone new repositories that have appeared
- Push the cloned repositories to a new GitLab instance, keeping the directory tree

## Install (TBD)

[//]: # (TODO: Write a complete installation guide)

```shell
cargo build --release
```

## Usage

```shell
gitlobster -t token -u url DESTINATION_DIR
```

## TODO

- [x] Clone all available repositories from a Gitlab instance to a group on another GitLab instance while preserving the
  directory structure
- [ ] Add include/exclude patterns
- [ ] Save not only the directory tree but also repository settings
- [ ] Add show option for only show repositories list
- [ ] Add config from file
- [ ] Add debug option
- [ ] Add cloning by http(s)
- [ ] Add parallel cloning
- [ ] Add tests

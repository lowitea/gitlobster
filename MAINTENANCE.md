# Maintenance guide

## Making a new release

1. Update master branch

   ```shell
   git checkout master && git pull
   ```

1. Update project version in `Cargo.toml`

   ```shell
   vim Cargo.toml
   ```

1. Commit `Cargo.toml` with the version

   ```shell
   git commit -m "bump version" Cargo.toml
   ```

1. Make a new git tag

   ```shell
   git tag <NEW_VERSION>
   ```

1. Push all to upstream

   ```shell
   git push origin master --follow-tags
   ```

1. [Create](https://github.com/lowitea/gitlobster/releases/new) a new release specifying pushed tag

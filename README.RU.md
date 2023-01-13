# Gitlobster

**_Инструмент полного копирования всех доступных репозиториев с сервера GitLab._**

[![Crates.io](https://img.shields.io/crates/v/gitlobster?style=for-the-badge)](https://crates.io/crates/gitlobster)
[![Docker Image Version (latest semver)](https://img.shields.io/docker/v/lowitea/gitlobster?sort=semver&label=docker&style=for-the-badge)](https://hub.docker.com/r/lowitea/gitlobster)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/lowitea/gitlobster/integration_test.yml?branch=master&label=integration%20tests&style=for-the-badge)](https://github.com/lowitea/gitlobster/actions)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/lowitea/gitlobster/test.yml?branch=master&label=unit%20tests&style=for-the-badge)](https://github.com/lowitea/gitlobster/actions)

[EN](README.md) | RU

<br>
<br>

<p align="center"><img src="https://github.com/lowitea/gitlobster/raw/master/logo.png"></p>

<br>

## Ключевые функции

- Клонирование всех доступных репозиториев
- Клонирование всех веток каждого репозитория
- Загрузка всех репозиториев в другой сервер GitLab (или в другую группу)
- Поддержка скачивания только обновлений (включая скачивание новых репозиториев), после первого полного клонирования
- Сохранение иерархии групп
- Поддержка фильтров (в том числе на основе регулярных выражений) для копирования только нужных репозиториев

## Установка

### Docker

```shell
docker run --rm -it lowitea/gitlobster:latest --help
```

### Запуск предварительно собранных бинарных файлов

1. Скачать собранный архив на [странице релизов](https://github.com/lowitea/gitlobster/releases) под свою платформу.
2. Распаковать архив.
3. Запустить через консоль файл `gitlobster`.

### Cargo

```shell
cargo install gitlobster
```

### Сборка из исходников

```shell
# клонирование репозитория
git clone https://github.com/lowitea/gitlobster.git
# переход в скачанную директорию
cd gitlobster
# сборка
cargo build --release
# запуск
./target/release/gitlobster --help
```

_Также есть возможность запустить в режиме разработки, без предварительной сборки._

```shell
# в директории с проектом
cargo run -- --help
```

## Использование

### GitLab токен

Для работы программы нужно сгенерировать токен GitLab с правами на чтение API (`read_api`). Если не используется копирование по SSH, также необходимы права на чтение репозиториев (`read_repository`).

Если репозитории копируются во второй GitLab, то токен нужен также и для него. Необходимы полные права на API (`api`). Если не используется загрузка по SSH, также необходимы права на запись репозиториев (`write_repository`).

Сгенерировать токены можно на [странице настроек](https://gitlab.com/-/profile/personal_access_tokens).

### SSH

Если используется копирование через SSH, тогда ssh-ключи должны быть [добавлены](https://gitlab.com/-/profile/keys) в GitLab.

### Вызов help

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
  -i, --include <PATTERN>          Include regexp patterns (cannot be used together with --exclude flag, may be repeated) [env: GTLBSTR_INCLUDE=]
  -x, --exclude <PATTERN>          Comma separated exclude regexp patterns (cannot be used together with --include flag, may be repeated) [env: GTLBSTR_EXCLUDE=]
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
      --only-master                Download only default branch [env: GTLBSTR_ONLY_MASTER=]
      --disable-sync-date          Disable adding sync dates in project descriptions [env: GTLBSTR_DISABLE_SYNC_DATE=]
  -h, --help                       Print help information
  -V, --version                    Print version information
```

### Копирование всех репозиториев в другой GitLab

```shell
gitlobster \
    --ft=<FETCH_TOKEN> \
    --fu=https://gitlab.com/ \
    --bt=<UPLOAD_TOKEN> \
    --bu=https://gitlab.com/ \
    --bg=gitlobster_test/upload
```

### Скачивание всех репозиториев в локальную папку

```shell
gitlobster \
    --ft=<FETCH_TOKEN> \
    --fu=https://gitlab.com/ \
    -d out_directory
```

_Поддерживается одновременное сохранение репозиториев локально и копирование во второй GitLab._

### Использование фильтров и фильтрующих флагов

```shell
gitlobster \
    --ft=<FETCH_TOKEN> \
    --fu=https://gitlab.com/ \
    --only-owned \
    --include="^gitlobster_test/download/project_2" \
    --include="^gitlobster_test/download/project_1" \
    -d out_directory
```

_Также поддерживается флаг `--exclude` для скачивания всех репозиториев, кроме тех, что подпадают под шаблон._

_Можно использовать только один из флагов `--exclude` или `--include`. При одновременном использовании этих флагов будет выдана соответствующая ошибка._

### Использование через докер

```shell
docker run --rm -it -v $(pwd)/out:/out lowitea/gitlobster:latest \
    --ft=<FETCH_TOKEN> \
    --fu=https://gitlab.com/ \
    --include='^gitlobster_test/example' \
    -d /out
```

### Предупреждение

Пустые проекты не будут перенесены. Так как, их невозможно склонировать через git, пока в этих проектах не будет создано ни одной ветки ([issue 37](https://github.com/lowitea/gitlobster/issues/37)).

## Аналоги

- [gitlabber](https://github.com/ezbz/gitlabber)
- [ghorg](https://github.com/gabrie30/ghorg)

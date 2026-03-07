runner := if x'${env:-local}' == "docker" {
    ""
  } else {
    "docker exec -it suffice-dev-1"
  }

all: build test lint

docker-build:
  docker compose build dev

up:
  {{ if runner != "" { `docker compose up -d` } else {''} }}

build: up
  {{runner}} cargo build

enter: up
  {{runner}} bash

lint: up
  {{runner}} cargo fmt
  {{runner}} cargo clippy --all-targets --no-deps -- -D warnings

test: up
  {{runner}} cargo test

clean:
  docker compose down

runner := if x'${env:-local}' == "docker" {
    ""
  } else {
    "docker exec -it suffice-dev-1"
  }

docker-build:
  docker compose build dev

up:
  {{ if runner != "" { `docker compose up -d` } else {''} }}

build: up
  {{runner}} cargo build

enter: up
  {{runner}} bash

lint: up
  {{runner}} cargo clippy --no-deps

clean:
  docker compose down

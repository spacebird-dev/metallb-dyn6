# use --release
release := "false"

_release_flag := if release == "true" { "--release" } else { "" }
_shared_args := "--all-features --workspace"

default: lint build test docs

lint: clippy format
clippy:
    cargo clippy {{ _shared_args }}
format:
    cargo fmt

build:
    cargo build {{ _shared_args }} {{ _release_flag }}
build-cross target:
    cross build {{ _shared_args }} --target {{ target }} {{ _release_flag }}

test:
    cargo test {{ _shared_args }} {{ _release_flag }}
test-cross target:
    cross test {{ _shared_args }} --target {{ target }} {{ _release_flag }}

docs:
    cargo doc --no-deps {{ _shared_args }}

docker tag: build
    docker buildx build --tag {{ tag }} .

run:
    cargo run -p metallb-dyn6

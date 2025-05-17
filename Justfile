shared_args := "--release --all-features --workspace"

default: format lint build

lint:
    cargo clippy
format:
    cargo fmt

run:
    cargo run -p metallb-dyn6

build:
    cargo build {{ shared_args }}
build-cross target:
    cross build {{ shared_args }} --target {{ target }}

test:
    cargo test {{ shared_args }}
test-cross target:
    cross test {{ shared_args }} --target {{ target }}

docker tag: build
    docker buildx build --tag {{ tag }} .

docs:
    cargo doc --no-deps --workspace

# metallb-dyn6

TODO

## Development

### Getting started

To begin development on this project, follow the steps below:

1. Clone this repository
2. Install cargo-make: `cargo install cargo-make --locked`
3. (Optional but recommended) Install [pre-commit](https://pre-commit.com/)
4. (Optional, for Docker builds and cross-compiling) Ensure docker is installed
4. Happy hacking!

This project uses an up-to-date version of Rust and `cargo-make` for builds.
Most common actions can be performed by running `cargo make <command>`.
See `cargo make --list-all-steps` for a list of available actions.

### Linting

- `cargo make lint`

### Building

To create a basic local build, just run:

`cargo make build`

For a release binary:

`cargo make -p release build`

This project uses [`cross`](https://github.com/cross-rs/cross) to cross-compile binaries for different target platforms.
You can create a binary for a different target like so:

- Debug: `cargo make build-aarch64-unknown-linux-gnu`
- Release: `cargo make -p release build-aarch64-unknown-linux-gnu`

To see which targets are available, run `cargo make --list-category-steps build`

### Tests

- Default (host) target: `cargo make test`
- Custom target: `cargo make test-aarch64-unknown-linux-gnu`
- Get a coverage report: `cargo make coverage`

### Docs

`cargo make docs`

### Docker

- To build a image for your local platform: `cargo make docker`
- Custom target: `cargo make docker-arm64`
    - Make sure your system can build [multi-platform images](https://docs.docker.com/build/building/multi-platform/) using QEMU
    - `cargo-make` will automatically generate an appropriate builder.
- To build the image with a custom tag: `cargo make -e DOCKER_TAG=registry.invalid/user/project:0.1.2 docker`

### Release Management

To create a new release, run the "Create Release PR" Action. This will generate a PR that you can then review and merge.
From there, the CI will automatically publish artifacts, etc.

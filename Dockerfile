FROM debian:bullseye-slim

ARG RUST_TARGET=""
ARG PROFILE_DIR

COPY target/${RUST_TARGET}/${PROFILE_DIR}/metallb-dyn6 /usr/local/bin/
RUN chmod +x /usr/local/bin/metallb-dyn6

# run unprivileged
USER 1001

CMD ["metallb-dyn6"]

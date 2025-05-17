FROM debian:bookworm-slim

ARG TARGET_DIR=release

COPY target/${TARGET_DIR}/metallb-dyn6 /usr/local/bin/
RUN chmod +x /usr/local/bin/metallb-dyn6

# run unprivileged
USER 1001

CMD ["metallb-dyn6"]

FROM rust:1.98-trixie AS builder
WORKDIR /src
COPY . .
RUN cargo build --release
COPY ./asset/ ./target/release/asset/
COPY ./start_server.sh ./target/release/start_server.sh
WORKDIR /src/target/release
RUN rm -rf ./build && rm -rf ./deps && rm -rf ./examples && rm -rf ./incremental
WORKDIR /src

FROM debian:trixie-slim
WORKDIR /root
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates wget unzip && \
    wget --no-verbose -O /tmp/chrome.deb https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb && \
    apt-get install -y --no-install-recommends /tmp/chrome.deb && \
    rm /tmp/chrome.deb
RUN wget https://chromedriver.storage.googleapis.com/111.0.5563.64/chromedriver_linux64.zip && \
    unzip chromedriver_linux64.zip && \
    mv chromedriver /usr/bin/chromedriver && \
    chown root:root /usr/bin/chromedriver && \
    chmod +x /usr/bin/chromedriver
RUN rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /src/target/release .
RUN ["chmod", "+x", "/app/start_server.sh"]
EXPOSE 80

ENTRYPOINT [ "/app/start_server.sh" ]

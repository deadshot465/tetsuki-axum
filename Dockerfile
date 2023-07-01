FROM rust:1.70 as builder
WORKDIR /src
COPY . .
RUN cargo build --release
COPY ./asset/ ./target/release/asset/
COPY ./start_server.sh ./target/release/start_server.sh
WORKDIR /src/target/release
RUN rm -rf ./build && rm -rf ./deps && rm -rf ./examples && rm -rf ./incremental
WORKDIR /src

FROM debian:buster-slim
WORKDIR /root
RUN apt-get update && \
    apt-get install -y apt-transport-https wget curl gnupg unzip
RUN curl -sS -o - https://dl-ssl.google.com/linux/linux_signing_key.pub | apt-key add && \
    echo "deb [arch=amd64]  http://dl.google.com/linux/chrome/deb/ stable main" >> /etc/apt/sources.list.d/google-chrome.list && \
    apt-get -y update
RUN wget --no-verbose -O /tmp/chrome.deb https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb && \
    apt install -y /tmp/chrome.deb && \
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

FROM rust:slim-bullseye as build

RUN apt-get update
RUN apt-get install pkg-config -y
RUN apt-get install openssl -y
RUN apt-get install libssl-dev -y

RUN USER=root cargo new --bin  /app/build/artifact-registry-proxy

WORKDIR /app/build/artifact-registry-proxy

COPY ./Cargo.toml ./
COPY ./Cargo.lock ./

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN cargo install --path ./ --target-dir /app/bin

FROM debian:bullseye-slim

COPY --from=build /app/bin/release/artifact-registry-proxy /app/data/

RUN apt-get update
RUN apt-get install pkg-config -y
RUN apt-get install openssl -y
RUN apt-get install libssl-dev -y
RUN apt-get install apt-transport-https ca-certificates gnupg curl -y

ENV CLOUDSDK_INSTALL_DIR /usr/local/gcloud/
RUN echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | tee -a /etc/apt/sources.list.d/google-cloud-sdk.list && curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | gpg --dearmor -o /usr/share/keyrings/cloud.google.gpg && apt-get update -y && apt-get install google-cloud-sdk google-cloud-cli python3 -y

RUN gcloud --version

EXPOSE 8000

WORKDIR /app/data
ENTRYPOINT ["/app/data/artifact-registry-proxy"]
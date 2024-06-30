[![Build Status](https://github.com/durganmcbroom/artifact-registry-proxy/actions/workflows/publish.yaml/badge.svg)](https://github.com/durganmcbroom/artifact-registry-proxy/actions)

# Artifact Registry Proxy 
(ARP)

An extremely fast and lightweight serverless Maven proxy designed to simplify authentication to Artifact Registry (GAR) in Google Cloud and allow for public access to GAR artifacts. This proxy was written in rust, runs on Docker, and is easily hosted on Cloud Run.

## Setup

### Environmental variables:

Artifact Registry Proxy requires the following 3 environmental variables to be in present:

 - `GAR_API_URL`: The URL to your pgk.dev project endpoint, eg. `https://<LOCATION>-maven.pkg.dev/<PROJECT_ID>`
 - `REPOSITORIES`: A comma split, colon pairing map of public repository names to internal GAR repositories. For example, `snapshots:my-projects-snapshots,releases:my-projects-releases` or `releases:releases1`
 - `CREDENTIALS`: A colon split user to key pair which will be used for all put operations on your repositories. ARP currently only supports Basic HTTP authentication and so will only accept a user and key value pair. For example: `my_user:a_very_secret_key`.

### GCloud

ARP Requires GCloud and an authentication token to be present in the environment while running. Make sure the GCloud command is working (and you are authenticated) if you wish to run locally. On Cloud Run Google will automatically inject service account credentials into the environment which the GCloud command and ARP will pick up. Your service account should have the following IAM permissions:
 - Artifact Registry Reader
 - Artifact Registry Writer
 - Service Account Token Creator
 - Secret Manager Secret Accessor (if using secrets for your credentials, highly recommended)

### Docker

To use this image without modification it's recommended to simply pull the image from our repository at `us-central1-docker.pkg.dev/extframework/docker-images`. You can use this directly in Cloud Run, or pull it locally and then push to your own host. Alternatively you could also build it yourself as detailed in the next section.

## Contributing

If you find a bug please report it as an issue and we will try to fix it ASAP. Otherwise, pull requests are greatly appreciated. 

### Building

Make sure you have `rustup` (at least 1.27.1) and have all necessary components installed. To build run `cargo build` or `cargo install --path ./`. To build with docker simple run (in the project directory) `docker build ./ <TAG OPTIONAL>` (gotta love the simplicity of containerization!)
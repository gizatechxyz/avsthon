use bollard::{
    container::Config, container::CreateContainerOptions, container::LogsOptions,
    container::StartContainerOptions, image::CreateImageOptions, Docker,
};
use eyre::Result;
use futures::StreamExt;
use regex::Regex;
use std::sync::Arc;

// TODO(eduponz): Handle manifest digest
pub struct DockerImageMetadata {
    pub repository: String,
    pub tag: String,
}

#[derive(Clone)]
pub(super) struct DockerClient {
    docker: Arc<Docker>,
}

impl DockerClient {
    pub fn new(docker: Arc<Docker>) -> Self {
        Self { docker }
    }

    pub async fn pull_image(&self, metadata: &DockerImageMetadata) -> Result<()> {
        // Download the image if we don't have it
        let options = CreateImageOptions {
            from_image: metadata.repository.clone(),
            tag: metadata.tag.clone(),
            ..Default::default()
        };

        // Request the image
        let mut stream = self.docker.create_image(Some(options), None, None);

        // Process the stream
        while let Some(result) = stream.next().await {
            match result {
                Ok(build_info) => {
                    if let Some(error) = build_info.error {
                        return Err(eyre::eyre!("Error pulling image: {:?}", error));
                    }
                }
                Err(e) => {
                    return Err(eyre::eyre!("Error pulling image: {:?}", e));
                }
            }
        }

        return Ok(());
    }

    pub async fn run_image(&self, metadata: &DockerImageMetadata) -> Result<String> {
        let container_opts = CreateContainerOptions {
            name: "test",
            ..Default::default()
        };

        let container_conf: Config<String> = Config {
            tty: Some(true),
            attach_stdin: Some(true),
            image: Some(metadata.repository.clone()),
            ..Default::default()
        };

        // TODO(eduponz): Handle error
        let container = self
            .docker
            .create_container(Some(container_opts), container_conf)
            .await
            .unwrap();

        // TODO(eduponz): Handle error
        self.docker
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await
            .unwrap();

        // TODO(eduponz): Find a better way to check if the container is done
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let log_opts = LogsOptions::<String> {
            stdout: true,
            ..Default::default()
        };

        let mut logs = self.docker.logs(&container.id, Some(log_opts));

        // Append logs to the task output
        let mut output = String::new();

        while let Some(log) = logs.next().await {
            // TODO(eduponz): Handle error
            output.push_str(&log.unwrap().to_string());
        }

        // TODO(eduponz): Handle error
        self.docker
            .stop_container(&container.id, None)
            .await
            .unwrap();

        // TODO(eduponz): Handle error
        self.docker
            .remove_container(&container.id, None)
            .await
            .unwrap();

        return Ok(output);
    }

    pub fn image_metadata(&self, dockerhub_url: &str) -> Result<DockerImageMetadata> {
        // Regex captures the repository, tag, and manifest digest from the URL
        let re = Regex::new(r"/layers/([^/]+/[^/]+)/([^/]+)/.+/sha256:([a-f0-9]+)").unwrap();

        let repository: String;
        let tag: String;

        if let Some(caps) = re.captures(dockerhub_url) {
            repository = caps[1].to_string();
            tag = caps[2].to_string();
        } else {
            return Err(eyre::eyre!(
                "No repository, tag, or digest found in URL: {:?}",
                dockerhub_url
            ));
        }

        let metadata = DockerImageMetadata {
            repository: repository,
            tag: tag,
        };

        Ok(metadata)
    }
}

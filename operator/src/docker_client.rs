use bollard::{
    container::Config, container::CreateContainerOptions, container::LogsOptions,
    container::StartContainerOptions, container::WaitContainerOptions, image::CreateImageOptions,
    Docker,
};
use eyre::Result;
use futures::StreamExt;
use regex::Regex;
use std::sync::Arc;

/// `DockerImageMetadata` holds metadata for a Docker image.
pub struct DockerImageMetadata {
    /// The Docker repository for the image.
    pub repository: String,
    /// The tag of the Docker image.
    pub tag: String,
}

/// `DockerClient` is a wrapper around the `Docker` struct provided by the `bollard` crate.
/// It provides functionality to interact with Docker, such as pulling images and running containers.
#[derive(Clone)]
pub(super) struct DockerClient {
    /// The underlying `Docker` client, wrapped in an `Arc` for shared ownership across threads.
    docker: Arc<Docker>,
}

impl DockerClient {
    /// Constructs a new `DockerClient` from an existing `Docker` client.
    ///
    /// # Arguments
    /// * `docker` - An `Arc<Docker>` object representing the Docker client.
    ///
    /// # Returns
    /// A new instance of `DockerClient`.
    pub fn new(docker: Arc<Docker>) -> Self {
        Self { docker }
    }

    /// Pulls a Docker image from the repository and tag specified in the `DockerImageMetadata`.
    ///
    /// This method streams the image download progress and handles any errors encountered during
    /// the process.
    ///
    /// # Arguments
    /// * `metadata` - A reference to `DockerImageMetadata` of the image to pull.
    ///
    /// # Errors
    /// Returns an `eyre::Result<()>` if the image pull encounters any errors or the stream reports an error.
    ///
    /// # Example
    /// ```rust
    /// let metadata = DockerImageMetadata { repository: "hello-world".to_string(), tag: "latest".to_string() };
    /// docker_client.pull_image(&metadata).await?;
    /// ```
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

        Ok(())
    }

    /// Runs a Docker image and retrieves the output logs.
    ///
    /// This method creates a container from the specified image, starts it, waits for it to exit,
    /// retrieves the logs, and then removes the container.
    ///
    /// # Arguments
    /// * `metadata` - A reference to `DockerImageMetadata` of the image to run.
    ///
    /// # Returns
    /// A `Result<String>` containing the container's output logs if successful.
    ///
    /// # Errors
    /// Returns an `eyre::Result<String>` if any step (container creation, start, wait, log retrieval, or container removal) fails.
    ///
    /// # Example
    /// ```rust
    /// let metadata = DockerImageMetadata { repository: "hello-world".to_string(), tag: "latest".to_string() };
    /// let output = docker_client.run_image(&metadata).await?;
    /// println!("Container output: {}", output);
    /// ```
    pub async fn run_image(&self, metadata: &DockerImageMetadata) -> Result<String> {
        // Create a container from the image
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

        let container = self
            .docker
            .create_container(Some(container_opts), container_conf)
            .await?;

        // Start the created container
        self.docker
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await?;

        // Wait for the container to exit
        let wait_opts = WaitContainerOptions {
            condition: "not-running",
            ..Default::default()
        };

        let mut wait_stream = self.docker.wait_container(&container.id, Some(wait_opts));

        while let Some(result) = wait_stream.next().await {
            match result {
                Ok(wait_info) => {
                    if let Some(error) = wait_info.error {
                        return Err(eyre::eyre!("Error waiting for container: {:?}", error));
                    }
                }
                Err(e) => {
                    return Err(eyre::eyre!("Error waiting for container: {:?}", e));
                }
            }
        }

        // Get the logs from the exited container
        let log_opts = LogsOptions::<String> {
            stdout: true,
            ..Default::default()
        };

        let mut logs = self.docker.logs(&container.id, Some(log_opts));

        let mut output = String::new();

        while let Some(log) = logs.next().await {
            output.push_str(&log?.to_string());
        }

        // Remove the exited container
        self.docker.remove_container(&container.id, None).await?;

        Ok(output)
    }

    /// Extracts the Docker image metadata (repository and tag) from a DockerHub URL.
    ///
    /// The method uses a regular expression to parse the URL and extract the repository and tag.
    /// If the URL does not contain a valid repository or tag, an error is returned.
    ///
    /// # Arguments
    /// * `dockerhub_url` - A string slice containing the DockerHub URL to parse.
    ///
    /// # Returns
    /// A `Result<DockerImageMetadata>` containing the parsed repository and tag.
    ///
    /// # Errors
    /// Returns an `eyre::Result<DockerImageMetadata>` if the URL does not contain valid repository or tag information.
    ///
    /// # Example
    /// ```rust
    /// let metadata = docker_client.image_metadata("https://hub.docker.com/layers/library/hello-world/latest/images/sha256:e2fc4e5")?;
    /// println!("Repository: {}, Tag: {}", metadata.repository, metadata.tag);
    /// ```
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

        let metadata = DockerImageMetadata { repository, tag };

        Ok(metadata)
    }
}

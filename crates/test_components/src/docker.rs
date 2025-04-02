use core::time;
use std::thread;

pub struct StartDockerCompose {}

impl StartDockerCompose {
    pub async fn start_docker_compose() -> bool {
        // TODO: to not need to hardcode the path to the docker-compose.yaml file
        let up_status = std::process::Command::new("docker-compose")
            .args(&[
                "-f",
                "../../mock_contracts/docker-compose.yaml",
                "up",
                "-d",
                "--build",
            ])
            .status()
            .expect("Failed to start docker-compose");

        let status = up_status.success();

        assert!(status, "Docker compose failed to start");

        let max_retries = 10;
        let delay = 5;
        let url = "http://127.0.0.1:5050/is_alive";
        let mut docker_compose_started = false;
        for _ in 0..max_retries {
            if let Ok(response) = reqwest::get(url).await {
                if response.status().is_success() {
                    docker_compose_started = true;
                    break;
                }
            }
            thread::sleep(time::Duration::from_secs(delay));
        }

        docker_compose_started
    }

    pub fn stop_docker_compose() -> bool {
        let down_status = std::process::Command::new("docker-compose")
            .args(&["-f", "../../mock_contracts/docker-compose.yaml", "down"])
            .status()
            .expect("Failed to stop docker-compose");

        let status = down_status.success();

        status
    }
}

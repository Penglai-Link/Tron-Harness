#![cfg(unix)]
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn termination_closes_incomplete_http_requests_and_releases_graph_for_restart() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let config = root.join("application.yaml");
    std::fs::write(&config, format!("bindAddress: 127.0.0.1\nport: {port}\nskillsRegistryRoot: {}/skills\nresource:\n  storage:\n    defaultProfile: local-default\n    profiles:\n      local-default:\n        type: local\n        root: {}/resources\n", root.display(), root.display())).unwrap();
    let log = root.join("server.log");
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(300))
        .build()
        .unwrap();
    for attempt in 0..2 {
        let output = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();
        let mut server = Server(
            Command::new(env!("CARGO_BIN_EXE_tron-backend"))
                .env("TRON_BACKEND_APPLICATION_CONFIG_PATH", &config)
                .env("TRON_BACKEND_DATABASE_URL", root.join("business.sqlite"))
                .env(
                    "TRON_BACKEND_CONTROL_DATABASE_URL",
                    root.join("profiles.sqlite"),
                )
                .env("TRON_BACKEND_GRAPH_STORAGE_PATH", root.join("graph.grafeo"))
                .env("TRON_BACKEND_PORT", port.to_string())
                .stdout(Stdio::from(output.try_clone().unwrap()))
                .stderr(Stdio::from(output))
                .spawn()
                .unwrap(),
        );
        let startup = Instant::now();
        loop {
            assert!(
                server.0.try_wait().unwrap().is_none(),
                "startup attempt {attempt} failed: {}",
                std::fs::read_to_string(&log).unwrap()
            );
            if client
                .get(format!("http://127.0.0.1:{port}/api/v1/health"))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                break;
            }
            assert!(
                startup.elapsed() < Duration::from_secs(15),
                "startup timed out"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        // Axum cannot finish a graceful drain while a request body never finishes.
        let mut incomplete = TcpStream::connect(("127.0.0.1", port)).unwrap();
        incomplete.write_all(b"POST /api/v1/rag/recall HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 10000\r\n\r\n{").unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let started = Instant::now();
        assert!(
            Command::new("kill")
                .args(["-TERM", &server.0.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
        loop {
            if let Some(status) = server.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "shutdown retained the process and file locks"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        drop(incomplete);
    }
    assert!(
        std::fs::read_to_string(log)
            .unwrap()
            .contains("shutdown requested:")
    );
}

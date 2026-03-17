pub mod ipc;
pub mod pinset;

pub use client::DaemonClient;

mod client {
    use super::ipc::{DaemonCommand, DaemonResponse, socket_path};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    pub struct DaemonClient {
        stream: UnixStream,
    }

    impl DaemonClient {
        /// Returns None if the daemon isn't running
        pub async fn connect() -> Option<Self> {
            UnixStream::connect(socket_path())
                .await
                .ok()
                .map(|stream| DaemonClient { stream })
        }

        pub fn is_running() -> bool {
            socket_path().exists()
        }

        pub async fn send(&mut self, cmd: DaemonCommand) -> anyhow::Result<DaemonResponse> {
            let mut line = serde_json::to_string(&cmd)?;
            line.push('\n');
            self.stream.write_all(line.as_bytes()).await?;

            let mut reader = BufReader::new(&mut self.stream);
            let mut response = String::new();
            reader.read_line(&mut response).await?;

            Ok(serde_json::from_str(&response)?)
        }
    }
}

use std::time::Duration;

use tokio::sync::mpsc;

use crate::frame::ServerFrame;

pub(crate) async fn create_heartbeat(interval: u32, channel: mpsc::Sender<ServerFrame>) {
    let mut timer = tokio::time::interval(Duration::from_millis(interval as u64 * 100));
    loop {
        timer.tick().await;
        if channel.send(ServerFrame::Heartbeat).await.is_err() {
            // Connection is closed
            return;
        }
    }
}

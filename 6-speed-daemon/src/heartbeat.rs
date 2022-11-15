use std::future::pending;
use std::time::Duration;

use anyhow::{bail, Result};
use tokio::time::{Instant, Interval};

#[derive(Debug, Default)]
pub struct Heartbeat {
    interval: Option<Interval>,
}

impl Heartbeat {
    pub fn set_interval(&mut self, interval: u32) -> Result<()> {
        match self.interval {
            None => {
                if interval != 0 {
                    let interval =
                        tokio::time::interval(Duration::from_millis(interval as u64 * 100));
                    self.interval = Some(interval);
                }
                Ok(())
            }
            Some(_) => bail!("Heartbeat already set"),
        }
    }

    pub async fn tick(&mut self) -> Instant {
        match &mut self.interval {
            None => pending().await,
            Some(interval) => interval.tick().await,
        }
    }
}

// impl Future for &Heartbeat {
//     type Output = Instant;
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         match &mut heartbeat.interval {
//             None => Poll::Pending,
//             Some(mut interval) => interval.poll_tick(cx),
//         }
//     }
// }

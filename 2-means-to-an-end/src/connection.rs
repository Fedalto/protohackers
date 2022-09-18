use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug)]
struct AssetPrice {
    pub timestamp: i32,
    pub price: i32,
}

impl AssetPrice {
    pub fn new(timestamp: i32, price: i32) -> Self {
        Self { timestamp, price }
    }
}

pub(crate) async fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0; 9];
    let mut historical_prices = Vec::new();
    loop {
        stream.read_exact(&mut buffer).await?;
        debug!("Received message: {:x?}", buffer);
        match buffer[0] as char {
            'I' => {
                let timestamp = i32::from_be_bytes(buffer[1..5].try_into().unwrap());
                let price = i32::from_be_bytes(buffer[5..9].try_into().unwrap());
                let asset_price = AssetPrice::new(timestamp, price);
                info!(
                    "Received new price. session={}, timestamp={}, price={}",
                    stream.peer_addr().unwrap(),
                    timestamp,
                    price
                );
                historical_prices.push(asset_price);
                historical_prices.sort_by_key(|asset| asset.timestamp);
            }
            'Q' => {
                let start_time = i32::from_be_bytes(buffer[1..5].try_into().unwrap());
                let end_time = i32::from_be_bytes(buffer[5..9].try_into().unwrap());

                if start_time > end_time {
                    // start_time comes after end_time. Must return 0 in this case.
                    stream.write_i32(0).await?;
                    continue;
                }

                let mut prices = Vec::new();
                for asset_price in &historical_prices {
                    if asset_price.timestamp < start_time {
                        continue;
                    }
                    if asset_price.timestamp > end_time {
                        break;
                    }
                    prices.push(asset_price.price);
                }
                if prices.is_empty() {
                    // There are no prices in the specified time range. Must return 0.
                    stream.write_i32(0).await?;
                } else {
                    let prices_sum = prices.iter().fold(0, |a, &b| a + b as i64);
                    let mean_price = prices_sum / prices.len() as i64;
                    stream.write_i32(mean_price as i32).await?;
                }
            }
            _ => panic!(),
        }
    }
}

use anyhow::Result;
use bytes::Bytes;

pub fn handle(request: Bytes) -> Result<()> {
    serde_json::from_slice(&request.to_vec())?;
    Ok(())
}

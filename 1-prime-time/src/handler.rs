use anyhow::{anyhow, bail, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Request {
    method: String,
    number: u64,
}

#[derive(Debug, Serialize)]
struct Response {
    method: String,
    prime: bool,
}

pub fn handle(request: Bytes) -> Result<Vec<u8>> {
    let request: Request =
        serde_json::from_slice(&request.to_vec()).or(Err(anyhow!("invalid json")))?;
    debug!("Received request: {request:?}");

    if request.method != "isPrime" {
        bail!("invalid method");
    }
    let response = Response {
        method: "isPrime".to_string(),
        prime: is_prime(request.number),
    };
    debug!("Sending response: {response:?}");

    Ok(serde_json::to_vec(&response)?)
}

fn is_prime(number: u64) -> bool {
    if number <= 1 {
        return false;
    }
    if number == 2 {
        return true;
    }
    if number % 2 == 0 {
        return false;
    }

    let limit = f64::sqrt(number as f64).trunc() as u64;
    for n in (3..limit).step_by(2) {
        if number % n == 0 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(21));
        assert!(is_prime(23));
    }
}

use std::str::FromStr;

use anyhow::{bail, Result};
use bytes::Bytes;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_json::Number;

#[derive(Debug, Deserialize, Serialize)]
struct Request {
    method: String,
    number: Number,
}

#[derive(Debug, Serialize)]
struct Response {
    method: String,
    prime: bool,
}

pub fn handle(request: Bytes) -> Result<Vec<u8>> {
    debug!("Received request: {}", String::from_utf8_lossy(&request));
    let request = match serde_json::from_slice::<Request>(&request.to_vec()) {
        Err(error) => {
            warn!(
                "Could not decode json. error={}, request={}",
                error,
                String::from_utf8_lossy(&request[..])
            );
            bail!("invalid json");
        }
        Ok(request) => request,
    };

    if request.method != "isPrime" {
        bail!("invalid method");
    }

    let is_prime = match BigInt::from_str(&request.number.to_string()) {
        Ok(bigint) => match bigint.to_u64() {
            // Abuse the fact that every BigInt sent is not prime
            None => false,
            Some(n) => is_prime(n),
        },
        // Failed to parse BigInt. It's a float, which is not prime.
        Err(_) => false,
    };

    let response = Response {
        method: "isPrime".to_string(),
        prime: is_prime,
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
    fn test_deserialize() {
        serde_json::from_str::<Request>(&r#"{"number":-3,"method":"isPrime"}"#).unwrap();
    }

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(21));
        assert!(is_prime(23));
    }

    #[test]
    fn test_handler_float() {
        let request_str = r#"{"method":"isPrime","number":3969458.1234}"#;
        let request = Bytes::from(request_str);

        let response = handle(request).unwrap();

        assert_eq!(
            r#"{"method":"isPrime","prime":false}"#,
            String::from_utf8_lossy(&response),
        );
    }

    #[test]
    fn test_handler_bigint() {
        let request_str = r#"{"method":"isPrime","number":33373922058321534863170207542241913960201772570479492638113619,"bignumber":true}"#;
        let request = Bytes::from(request_str);
        let response = handle(request).unwrap();
        assert_eq!(
            r#"{"method":"isPrime","prime":false}"#,
            String::from_utf8_lossy(&response),
        );
    }
}

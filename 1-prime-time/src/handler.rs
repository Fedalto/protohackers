use anyhow::{bail, Result};
use bytes::Bytes;
use num_bigint::{BigInt, ToBigInt};
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
struct Request {
    method: String,
    #[serde(deserialize_with = "deserialize_number")]
    number: BigInt,
}

fn deserialize_number<'de, D>(deserializer: D) -> Result<BigInt, D::Error>
where
    D: Deserializer<'de>,
{
    let number = serde_json::Number::deserialize(deserializer)?;
    Ok(BigInt::from_str(&number.to_string()).unwrap())
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
    let response = Response {
        method: "isPrime".to_string(),
        prime: is_prime(request.number),
    };
    debug!("Sending response: {response:?}");

    Ok(serde_json::to_vec(&response)?)
}

fn is_prime(number: BigInt) -> bool {
    if number <= 1.to_bigint().unwrap() {
        return false;
    }
    if number == 2.to_bigint().unwrap() || number == 3.to_bigint().unwrap() {
        return true;
    }
    if &number % 2.to_bigint().unwrap() == 0.to_bigint().unwrap() {
        return false;
    }
    let limit = number.sqrt();
    let mut n = 3.to_bigint().unwrap();
    loop {
        if &number % &n == 0.to_bigint().unwrap() {
            return false;
        }
        n += 2.to_bigint().unwrap();
        if n > limit {
            break;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let request: Request =
            serde_json::from_str(&r#"{"number":-3,"method":"isPrime"}"#).unwrap();
        assert_eq!(
            request.number,
            BigInt::new(num_bigint::Sign::Minus, vec![3])
        );
    }

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(1.to_bigint().unwrap()));
        assert!(is_prime(2.to_bigint().unwrap()));
        assert!(is_prime(3.to_bigint().unwrap()));
        assert!(!is_prime(21.to_bigint().unwrap()));
        assert!(is_prime(23.to_bigint().unwrap()));
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

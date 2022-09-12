use bytes::{Bytes, BytesMut};

#[derive(Debug, PartialEq)]
pub struct Frame(pub Bytes);

impl Frame {
    pub fn parse(buffer: &BytesMut) -> Option<(Self, usize)> {
        for (position, &byte) in buffer.iter().enumerate() {
            if byte == b'\n' {
                let frame = Frame(Bytes::copy_from_slice(&buffer[0..position]));
                return Some((frame, position + 1));
            }
        }
        None
    }
}

impl From<&[u8]> for Frame {
    fn from(input: &[u8]) -> Self {
        Self(Bytes::copy_from_slice(input))
    }
}

impl From<Vec<u8>> for Frame {
    fn from(input: Vec<u8>) -> Self {
        Self(Bytes::from(input))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BufMut;

    use super::*;

    #[test]
    fn test_parse_frame() {
        let mut buffer = BytesMut::new();

        buffer.put(&b"{\"method\":\"isPrime\","[..]);
        assert!(Frame::parse(&buffer).is_none());

        buffer.put(&b"\"number\":1}\n"[..]);
        let (frame, _) = Frame::parse(&buffer).unwrap();
        assert_eq!(
            frame,
            Frame::from(&b"{\"method\":\"isPrime\",\"number\":1}"[..])
        );
    }
}

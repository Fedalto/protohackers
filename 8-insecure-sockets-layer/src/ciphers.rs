use std::fmt::Debug;

pub trait Cipher: Debug + Send {
    fn apply(&self, buf: &[u8], position: u64) -> Vec<u8>;

    fn reverse(&self, buf: &[u8], position: u64) -> Vec<u8> {
        self.apply(buf, position)
    }
}

#[derive(Debug)]
pub struct ReverseBits;

impl Cipher for ReverseBits {
    fn apply(&self, buf: &[u8], _position: u64) -> Vec<u8> {
        buf.iter().map(|&byte| byte.reverse_bits()).collect()
    }
}

#[derive(Debug)]
pub struct XorN(u8);

impl XorN {
    pub fn new(n: u8) -> Self {
        Self(n)
    }
}

impl Cipher for XorN {
    fn apply(&self, buf: &[u8], _position: u64) -> Vec<u8> {
        buf.iter().map(|byte| byte ^ self.0).collect()
    }
}

#[derive(Debug)]
pub struct XorPos;

impl Cipher for XorPos {
    fn apply(&self, buf: &[u8], position: u64) -> Vec<u8> {
        buf.iter()
            .enumerate()
            .map(|(pos, byte)| byte ^ ((pos as u64 + position) as u8))
            .collect()
    }
}

#[derive(Debug)]
pub struct AddN(u8);

impl AddN {
    pub fn new(n: u8) -> Self {
        Self(n)
    }
}

impl Cipher for AddN {
    fn apply(&self, buf: &[u8], _position: u64) -> Vec<u8> {
        buf.iter().map(|byte| byte.wrapping_add(self.0)).collect()
    }

    fn reverse(&self, buf: &[u8], _position: u64) -> Vec<u8> {
        buf.iter().map(|byte| byte.wrapping_sub(self.0)).collect()
    }
}

#[derive(Debug)]
pub struct AddPos;

impl Cipher for AddPos {
    fn apply(&self, buf: &[u8], position: u64) -> Vec<u8> {
        buf.iter()
            .enumerate()
            .map(|(i, byte)| byte.wrapping_add(i as u8).wrapping_add(position as u8))
            .collect()
    }

    fn reverse(&self, buf: &[u8], position: u64) -> Vec<u8> {
        buf.iter()
            .enumerate()
            .map(|(i, byte)| byte.wrapping_sub(i as u8).wrapping_sub(position as u8))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_pos_cipher() {
        let add_pos = AddPos;
        let message = vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]; // hello

        let cipher_text = add_pos.apply(&message, 0);
        assert_eq!(cipher_text, vec![0x68, 0x66, 0x6e, 0x6f, 0x73]);
        let cipher_text = add_pos.apply(&cipher_text, 0);
        assert_eq!(cipher_text, vec![0x68, 0x67, 0x70, 0x72, 0x77]);

        let cipher_text = add_pos.reverse(&cipher_text, 0);
        assert_eq!(cipher_text, vec![0x68, 0x66, 0x6e, 0x6f, 0x73]);
        let cipher_text = add_pos.reverse(&cipher_text, 0);
        assert_eq!(cipher_text, message);
    }

    #[test]
    fn test_add_pos_with_position() {
        let add_pos = AddPos;
        let message = vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]; // hello
        assert_eq!(
            add_pos.apply(&message, 2),
            vec![0x6a, 0x68, 0x70, 0x71, 0x75]
        );
    }
}

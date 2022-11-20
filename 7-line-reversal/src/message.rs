use once_cell::sync::Lazy;
use regex::Regex;

pub type SessionId = u32;

#[derive(Debug)]
pub struct InvalidMessage(&'static str);

#[derive(Debug, Eq, PartialEq)]
pub enum Message {
    Connect(SessionId),
    Data {
        session: SessionId,
        position: u32,
        data: String,
    },
    Ack {
        session: SessionId,
        position: u32,
    },
    Disconnect(SessionId),
}

impl Message {
    pub fn session_id(&self) -> SessionId {
        *match self {
            Message::Connect(session) => session,
            Message::Data { session, .. } => session,
            Message::Ack { session, .. } => session,
            Message::Disconnect(session) => session,
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            Message::Connect(session_id) => format!("/connect/{session_id}/"),
            Message::Data {
                session,
                position,
                data,
            } => {
                let escaped_data = data.replace("/", r"\/").replace(r"\", r"\\");
                format!("/data/{session}/{position}/{escaped_data}/")
            }
            Message::Ack { session, position } => format!("/ack/{session}/{position}/"),
            Message::Disconnect(session) => format!("/close/{session}/"),
        }
        .as_bytes()
        .to_vec()
    }
}

impl TryFrom<&[u8]> for Message {
    type Error = InvalidMessage;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        static INVALID_DATA_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^\\]/").unwrap());

        fn parse_int(
            parts: &[&str],
            index: usize,
            error_message: &'static str,
        ) -> Result<u32, InvalidMessage> {
            parts
                .get(index)
                .ok_or(InvalidMessage(error_message))?
                .parse::<u32>()
                .map_err(|_| InvalidMessage(error_message))
        }

        let s = String::from_utf8(value.to_vec())
            .map_err(|_| InvalidMessage("Could not decode UTF-8"))?;
        debug!("Parsing new message: {s:?}");
        if !s.is_ascii() {
            return Err(InvalidMessage("Message is not ascii"));
        }
        let s = s
            .strip_prefix("/")
            .ok_or(InvalidMessage("Message don't start with /"))?;
        let s = s
            .strip_suffix("/")
            .ok_or(InvalidMessage("Message don't end with /"))?;

        let parts: Vec<&str> = s.splitn(4, "/").collect();
        let message_type = parts
            .first()
            .ok_or(InvalidMessage("Could not extract message type"))?;

        match *message_type {
            "connect" => {
                let session = parse_int(&parts, 1, "Could not parse session ID")?;
                Ok(Message::Connect(session))
            }

            "data" => {
                let session = parse_int(&parts, 1, "Could not parse session ID")?;
                let position = parse_int(&parts, 2, "Could not parse position")?;
                let data = parts.get(3).ok_or(InvalidMessage("Could not parse data"))?;
                if INVALID_DATA_REGEX.is_match(data) {
                    return Err(InvalidMessage("Data should escape /"));
                }
                let data = data.replace(r"\/", "/");
                let data = data.replace(r"\\", r"\");
                Ok(Message::Data {
                    session,
                    position,
                    data: data.to_string(),
                })
            }

            "ack" => {
                let session = parse_int(&parts, 1, "Could not parse session ID")?;
                let position = parse_int(&parts, 2, "Could not parse position")?;
                Ok(Message::Ack { session, position })
            }

            "close" => {
                let session = parse_int(&parts, 1, "Could not parse session ID")?;
                Ok(Message::Disconnect(session))
            }

            _ => Err(InvalidMessage("Invalid message type")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_message() {
        let packet = b"/connect/1234567/";
        assert_eq!(
            Message::try_from(&packet[..]).unwrap(),
            Message::Connect(1234567)
        );

        let packet = b"/data/1234567/0/hello/";
        assert_eq!(
            Message::try_from(&packet[..]).unwrap(),
            Message::Data {
                session: 1234567,
                position: 0,
                data: "hello".to_string(),
            }
        );

        let packet = b"/ack/1234567/1024/";
        assert_eq!(
            Message::try_from(&packet[..]).unwrap(),
            Message::Ack {
                session: 1234567,
                position: 1024,
            }
        );

        let packet = b"/close/1234567/";
        assert_eq!(
            Message::try_from(&packet[..]).unwrap(),
            Message::Disconnect(1234567)
        );
    }

    #[test]
    fn test_data_escape() {
        let packet = br"/data/782831017/3/\/bar\/baz\nfoo\\bar\\baz\n/";
        assert_eq!(
            Message::try_from(&packet[..]).unwrap(),
            Message::Data {
                session: 782831017,
                position: 3,
                data: r"/bar/baz\nfoo\bar\baz\n".to_string(),
            }
        );

        let packet = br"/data/1805784010/41/illegal data/has too many/parts/";
        assert!(Message::try_from(&packet[..]).is_err());
    }
}

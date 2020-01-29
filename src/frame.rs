use crate::message::Message;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bytes::{BytesMut, *};
use std::io::Cursor;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug)]
pub enum Opcode {
    Continue,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
    Unknown,
}

impl From<u8> for Opcode {
    fn from(lh: u8) -> Self {
        match lh {
            0x00 => Self::Continue,
            0x01 => Self::Text,
            0x02 => Self::Binary,
            0x08 => Self::Close,
            0x09 => Self::Ping,
            0xA0 => Self::Pong,
            _ => Self::Unknown,
        }
    }
}

impl Into<u8> for Opcode {
    fn into(self) -> u8 {
        match self {
            Self::Continue => 0x00,
            Self::Text => 0x01,
            Self::Binary => 0x02,
            Self::Close => 0x08,
            Self::Ping => 0x09,
            Self::Pong => 0xA0,
            Self::Unknown => 0xB0,
        }
    }
}

#[derive(Debug)]
pub struct Frame {
    fin: bool,
    rsv1: bool,
    rsv2: bool,
    rsv3: bool,
    opcode: Opcode,
    masked: bool,
    length: u64,
    key: Vec<u8>,
    data: Vec<u8>,
    message: String,
}

impl Frame {
    pub fn new(message: String) -> Self {
        Self {
            message: message.clone(),
            length: message.len() as u64,
            data: message.as_bytes().to_vec(),
            ..Default::default()
        }
    }

    pub fn get_msg(&self) -> String {
        self.message.clone()
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame {
            fin: true,
            rsv1: false,
            rsv2: false,
            rsv3: false,
            opcode: Opcode::Text,
            masked: false,
            length: 0,
            key: vec![0, 0, 0, 0],
            data: vec![],
            message: "".into(),
        }
    }
}

#[derive(Debug)]
pub struct WebsocketFrame;

impl WebsocketFrame {
    pub fn mutate(data: &[u8], key: &[u8]) -> Vec<u8> {
        data.iter()
            .zip(key.iter().cycle())
            .map(|(b, k)| b ^ k)
            .collect()
    }
}

impl Decoder for WebsocketFrame {
    type Item = Frame;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut pos = 0;
        if src.len() < 2 {
            return Ok(None);
        }

        let head = &src[..2];
        pos += 2;

        let first = head[0];
        let second = head[1];

        let fin = first & 0x80 != 0;
        let rsv1 = first & 0x40 != 0;
        let rsv2 = first & 0x20 != 0;
        let rsv3 = first & 0x10 != 0;
        let opcode = first & 0x0f;
        let masked = second & 0x80 != 0;

        if !masked {
            src.clear();
            return Ok(None);
        }

        let length = (second & 0x7f) as u64;

        let length = if length == 126 {
            let mut rdr = Cursor::new(&src[2..4]);
            pos += 2;
            rdr.read_u16::<BigEndian>()? as u64
        } else if length == 127 {
            let mut rdr = Cursor::new(&src[2..10]);
            pos += 8;
            rdr.read_u64::<BigEndian>()? as u64
        } else {
            length
        };

        let key = &src[pos..pos + 4];
        pos += 4;

        let data = &src[pos..pos + length as usize];
        let decoded = WebsocketFrame::mutate(data, key);
        let string_form = String::from_utf8_lossy(&decoded);

        let item = Some(Self::Item {
            fin,
            rsv1,
            rsv2,
            rsv3,
            opcode: opcode.into(),
            masked,
            length,
            key: key.to_vec(),
            data: decoded.clone(),
            message: string_form.as_ref().to_string(),
        });
        src.clear();

        Ok(item)
    }
}

impl Encoder for WebsocketFrame {
    type Item = Frame;
    type Error = std::io::Error;

    // TODO: Add support for chunked frames.
    fn encode(&mut self, frame: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let mut frame = frame;
        let mut one = 0u8 | 0x80;
        if frame.rsv1 {
            one |= 0x40;
        }

        if frame.rsv2 {
            one |= 0x20;
        }

        if frame.rsv3 {
            one |= 0x10;
        }

        one |= frame.opcode as u8;

        let mut two = 0u8;
        frame.masked = false;

        if frame.masked {
            two |= 0x80;
        }

        match frame.data.len() {
            len if len < 126 => {
                buf.reserve(2);
                two |= len as u8;
            }
            len if len <= 65535 => {
                buf.reserve(4);
                two |= 126;
            }
            _ => {
                buf.reserve(10);
                two |= 127;
            }
        }

        buf.put_slice(&[one, two]);

        if let Some(length_bytes) = match frame.data.len() {
            len if len < 126 => None,
            len if len <= 65535 => Some(2),
            _ => Some(8),
        } {
            let mut rdr = Cursor::new(Vec::new());
            buf.reserve(length_bytes);
            rdr.write_uint::<BigEndian>(frame.data.len() as u64, length_bytes)?;
            buf.put_slice(rdr.into_inner().as_ref());
        }

        buf.reserve(frame.data.len());

        if frame.masked {
            buf.reserve(frame.key.len());
            buf.put_slice(&frame.key);
            buf.put_slice(&WebsocketFrame::mutate(&frame.data, &frame.key));
        } else {
            buf.put_slice(&frame.data);
        }
        Ok(())
    }
}

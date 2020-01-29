use crate::frame::Frame;

pub struct Message {
    data: String,
}

impl Message {
    pub fn new(data: String) -> Self {
        Self { data }
    }

    pub fn from<T: Into<String>>(a: T) -> Self {
        Self { data: a.into() }
    }

    pub fn from_frame(a: &Frame) -> Self {
        Self { data: a.get_msg() }
    }
}

impl Into<Frame> for Message {
    fn into(self) -> Frame {
        Frame::new(self.data.into())
    }
}

use crate::frame::Frame;

pub struct Message {
    data: String,
}

impl Message {
    pub fn from<T: Into<String>>(a: T) -> Self {
        Self { data: a.into() }
    }
}

impl Into<Frame> for Message {
    fn into(self) -> Frame {
        Frame::new(self.data.into())
    }
}

use crate::{
    DecodingError,
    header::{ChannelMode, FrameHeader},
    read_u32,
};

#[derive(Debug)]
pub struct SideInfo {}

impl SideInfo {
    pub fn len(channel_mode: ChannelMode) -> usize {
        if channel_mode == ChannelMode::Mono {
            17
        } else {
            32
        }
    }

    pub fn read(header: &FrameHeader, frame_data: &[u8]) -> Result<Self, DecodingError> {
        let offset = SideInfo::len(header.channel_mode);
        let mut side_info_bytes = &frame_data[..offset];
        let common = read_u32(&mut side_info_bytes);

        Ok(SideInfo {})
    }
}

#[cfg(test)]
mod tests {
    use crate::Frame;

    use super::*;
    use std::fs::read;

    #[test]
    fn test_side_info() {
        let data = read("tests/sine_320hz_50ms_vbr_frame1-3.mp3").unwrap();
        let (header, frame_data) = Frame::read_header(&data).unwrap();
        let side_info = SideInfo::read(&header, &frame_data).unwrap();
    }
}

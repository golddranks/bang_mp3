use crate::{
    DecodingError,
    header::{FrameHeader, Version},
    read_u32,
    side_info::SideInfo,
};

#[derive(Debug, Default)]
pub struct VbrInfo {
    frames: Option<u32>,
    filesize: Option<u32>,
    toc: Option<Box<[u8; 100]>>,
    vbr_scale: Option<u32>,
}

impl VbrInfo {
    fn read_info(mut data: &[u8]) -> Result<Self, DecodingError> {
        let tags = read_u32(&mut data)?;
        let frames = tags & 1 == 1;
        let filesize = tags & 2 == 2;
        let toc = tags & 4 == 4;
        let vbr_scale = tags & 8 == 8;

        let mut vbr_info = Self::default();

        dbg!(tags);

        if frames {
            vbr_info.frames = Some(read_u32(&mut data)?);
        }
        if filesize {
            vbr_info.filesize = Some(read_u32(&mut data)?);
        }
        if toc {
            vbr_info.toc = Some(Box::new(
                data[..100]
                    .try_into()
                    .map_err(|_| DecodingError::UnexpectedEndOfStream)?,
            ));
        }
        data = &data[100..];
        if vbr_scale {
            vbr_info.vbr_scale = Some(read_u32(&mut data)?);
        }

        Ok(vbr_info)
    }

    pub fn read(header: &FrameHeader, data: &[u8]) -> Option<Result<Self, DecodingError>> {
        let mut data = match header.version {
            Version::MPEG1 => &data[SideInfo::len(header)..],
            _ => return Some(Err(DecodingError::UnsupportedVersion)),
        };
        if read_u32(&mut data) != Ok(u32::from_be_bytes(*b"Xing")) {
            return None;
        }
        Some(Self::read_info(data))
    }
}

#[cfg(test)]
mod tests {
    use crate::Frame;

    use super::*;
    use std::fs::read;

    #[test]
    fn test_vbr_info() {
        let data = read("tests/sine_320hz_50ms_vbr_frame0.mp3").unwrap();
        let (header, frame_data) = Frame::read_header(&data).unwrap();
        let vbr_info = VbrInfo::read(&header, &frame_data).unwrap().unwrap();
        assert_eq!(vbr_info.frames, Some(3));
        assert_eq!(vbr_info.filesize, Some(1643));
        assert_eq!(
            vbr_info.toc,
            Some(Box::new([
                0, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152,
                152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152, 152,
                152, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179,
                179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179, 179,
                179, 179, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255
            ]))
        );
        assert_eq!(vbr_info.vbr_scale, Some(80));
    }
}

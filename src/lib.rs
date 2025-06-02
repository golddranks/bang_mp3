use std::ops::{Range, Shl, Shr};

use header::FrameHeader;
use side_info::SideInfo;
use vbr::VbrInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodingError {
    UnexpectedEndOfStream,
    InvalidFrameHeader,
    UnsupportedVersion,
    UnsupportedLayer,
    UnsupportedBitrate,
    UnsupportedSamplingRate,
    UnsupportedEmphasis,
    InvalidBlockType,
}

mod decoder;
mod header;
mod side_info;
mod vbr;

fn read_u16(data: &mut &[u8]) -> Result<u16, DecodingError> {
    let int = u16::from_be_bytes(
        data[..2]
            .try_into()
            .map_err(|_| DecodingError::UnexpectedEndOfStream)?,
    );
    *data = &data[2..];
    Ok(int)
}

fn read_u32(data: &mut &[u8]) -> Result<u32, DecodingError> {
    let int = u32::from_be_bytes(
        data[..4]
            .try_into()
            .map_err(|_| DecodingError::UnexpectedEndOfStream)?,
    );
    *data = &data[4..];
    Ok(int)
}

fn read_u64(data: &mut &[u8]) -> Result<u64, DecodingError> {
    let int = u64::from_be_bytes(
        data[..8]
            .try_into()
            .map_err(|_| DecodingError::UnexpectedEndOfStream)?,
    );
    *data = &data[8..];
    Ok(int)
}

fn read_bits<T>(val: T, bits: Range<u8>) -> T
where
    T: Shl<u8, Output = T> + Shr<u8, Output = T>,
{
    (val << bits.start) >> (bits.start + size_of::<T>() as u8 * 8 - bits.end)
}

pub enum FirstFrame<'a> {
    Vbr(FrameHeader, VbrInfo),
    Cbr(Frame<'a>),
}

impl FirstFrame<'_> {
    pub fn len(&self) -> usize {
        match self {
            FirstFrame::Vbr(header, _) => header.frame_bytes,
            FirstFrame::Cbr(frame) => frame.header.frame_bytes,
        }
    }
}

pub struct Frame<'a> {
    pub header: header::FrameHeader,
    pub side_info: side_info::SideInfo,
    pub main_data: &'a [u8],
}

impl<'a> Frame<'a> {
    fn read_header(data: &'a [u8]) -> Result<(FrameHeader, &'a [u8]), DecodingError> {
        if data.len() < 4 {
            return Err(DecodingError::UnexpectedEndOfStream);
        }

        let header = match FrameHeader::read(data) {
            Ok(header) => header,
            Err(err) => return Err(err),
        };

        if data.len() < header.frame_bytes {
            return Err(DecodingError::UnexpectedEndOfStream);
        }

        let frame_data = &data[header.len()..header.frame_bytes];

        Ok((header, frame_data))
    }

    fn read_frame_data(
        header: FrameHeader,
        frame_data: &'a [u8],
    ) -> Result<Frame<'a>, DecodingError> {
        let side_info_len = SideInfo::len(&header);
        let side_info = SideInfo::read(&header, &frame_data[..side_info_len])?;
        let main_data = &frame_data[side_info_len..];

        Ok(Frame {
            header,
            side_info,
            main_data,
        })
    }

    pub fn read_first(data: &'a [u8]) -> Result<FirstFrame<'a>, DecodingError> {
        let (header, frame_data) = Frame::read_header(data)?;

        if let Some(vbr_info) = VbrInfo::read(&header, &frame_data) {
            Ok(FirstFrame::Vbr(header, vbr_info?))
        } else {
            Ok(FirstFrame::Cbr(Self::read_frame_data(header, frame_data)?))
        }
    }

    pub fn read(data: &'a [u8]) -> Result<Self, DecodingError> {
        let (header, frame_data) = Frame::read_header(data)?;
        Self::read_frame_data(header, frame_data)
    }
}

pub struct FrameIter<'a> {
    data: &'a [u8],
}

impl<'a> FrameIter<'a> {
    pub fn new(data: &'a [u8]) -> Result<(FirstFrame<'a>, Self), DecodingError> {
        let first_frame = Frame::read_first(data)?;
        let consumed = first_frame.len();
        Ok((
            first_frame,
            FrameIter {
                data: &data[consumed..],
            },
        ))
    }
}

impl<'a> Iterator for FrameIter<'a> {
    type Item = Result<Frame<'a>, DecodingError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() < 4 {
            return None;
        }

        Some(Frame::read(self.data).inspect(|frame| {
            self.data = &self.data[frame.header.frame_bytes..];
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read;

    #[test]
    fn test_frame_iter_short() {
        let data = read("tests/sine_320hz_50ms_vbr.mp3").unwrap();
        let (first_frame, iter) = FrameIter::new(&data).unwrap();
        assert!(matches!(first_frame, FirstFrame::Vbr(_, _)));

        let expected_lengths = vec![731, 130, 365, /* EOS */ 9999];

        for (frame, expected_len) in iter.zip(expected_lengths.iter()) {
            let Frame { header, .. } = frame.unwrap();
            assert_eq!(header.sampling_rate, 44100);
            assert_eq!(header.frame_bytes, *expected_len);
        }
    }

    #[test]
    fn test_frame_iter() {
        let data = read("tests/sine_440hz_500ms_vbr.mp3").unwrap();
        let (first_frame, iter) = FrameIter::new(&data).unwrap();
        assert!(matches!(first_frame, FirstFrame::Vbr(_, _)));

        let expected_bitrates = vec![
            224, 48, 40, 40, 32, 40, 32, 32, 40, 32, 40, 32, 32, 40, 32, 32, 32, 32, 32, 128, 32,
            /* EOS */ 9999,
        ];

        for (frame, expected_bitrate) in iter.zip(expected_bitrates.iter()) {
            let Frame { header, .. } = frame.unwrap();
            assert_eq!(header.sampling_rate, 44100);
            assert_eq!(header.bitrate, *expected_bitrate);
        }
    }

    #[test]
    fn test_read_bits() {
        assert_eq!(read_bits(0xFFFFFFFF00000000, 0..32), 0xFFFFFFFF_u64);
        assert_eq!(read_bits(0x00000000FFFFFFFF, 32..64), 0xFFFFFFFF_u64);
        assert_eq!(read_bits(0x0000FFFFFFFF0000, 16..48), 0xFFFFFFFF_u64);
        assert_eq!(read_bits(0x00000000FFFFFFFF, 0..32), 0x00000000_u64);
        assert_eq!(read_bits(0xFFFFFFFF00000000, 32..64), 0x00000000_u64);
        assert_eq!(read_bits(0xFFFF00000000FFFF, 16..48), 0x00000000_u64);
    }

    #[test]
    fn test_read_u16() {
        let mut data = b"\xAB\xCD".as_slice();
        assert_eq!(read_u16(&mut data).unwrap(), 0xABCD_u16);
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_read_u32() {
        let mut data = b"\x89\xAB\xCD\xEF".as_slice();
        assert_eq!(read_u32(&mut data).unwrap(), 0x89AB_CDEF_u32);
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_read_u64() {
        let mut data = b"\x89\xAB\xCD\xEF\x01\x23\x45\x67".as_slice();
        assert_eq!(read_u64(&mut data).unwrap(), 0x89AB_CDEF_0123_4567_u64);
        assert_eq!(data.len(), 0);
    }
}

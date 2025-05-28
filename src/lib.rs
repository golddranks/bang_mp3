#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodingError {
    UnexpectedEndOfStream,
    InvalidFrameHeader,
    UnsupportedVersion,
    UnsupportedLayer,
    UnsupportedBitrate,
    UnsupportedSamplingRate,
    UnsupportedEmphasis,
}

mod decoder;
mod header;

pub struct Frame<'a> {
    pub header: header::FrameHeader,
    pub data: &'a [u8],
}

impl<'a> Frame<'a> {
    pub fn read(data: &'a [u8]) -> Result<Self, DecodingError> {
        if data.len() < 4 {
            return Err(DecodingError::UnexpectedEndOfStream);
        }

        let header = match header::read(data) {
            Ok(header) => header,
            Err(err) => return Err(err),
        };

        let frame_len = header.frame_bytes as usize;
        if data.len() < frame_len {
            return Err(DecodingError::UnexpectedEndOfStream);
        }
        let frame_data = &data[..frame_len];
        Ok(Frame {
            header,
            data: frame_data,
        })
    }
}

pub struct FrameIter<'a> {
    data: &'a [u8],
}

impl<'a> FrameIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        FrameIter { data }
    }
}

impl<'a> Iterator for FrameIter<'a> {
    type Item = Result<Frame<'a>, DecodingError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() < 4 {
            return None;
        }

        Some(Frame::read(self.data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read;

    #[test]
    fn test_frame_iter_short() {
        let data = read("tests/sine_320hz_50ms.mp3").unwrap();
        let iter = FrameIter::new(&data);

        let expected_lengths = vec![417, 731, 130, 365, /* EOS */ 9999];

        for (frame, expected_len) in iter.zip(expected_lengths.iter()) {
            let Frame { header, .. } = frame.unwrap();
            assert_eq!(header.sampling_rate, 44100);
            assert_eq!(header.frame_bytes, *expected_len);
        }
    }

    #[test]
    fn test_frame_iter() {
        let data = read("tests/sine_440hz_500ms.mp3").unwrap();
        let iter = FrameIter::new(&data);

        let expected_bitrates = vec![
            128, 224, 48, 40, 40, 32, 40, 32, 32, 40, 32, 40, 32, 32, 40, 32, 32, 32, 32, 32, 128,
            32, /* EOS */ 9999,
        ];

        for (frame, expected_bitrate) in iter.zip(expected_bitrates.iter()) {
            let Frame { header, .. } = frame.unwrap();
            assert_eq!(header.sampling_rate, 44100);
            assert_eq!(header.bitrate, *expected_bitrate);
        }
    }
}

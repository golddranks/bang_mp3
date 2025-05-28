use crate::DecodingError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    Stereo,
    JointStereo,
    DualChannel,
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    None,
    FiftyFifteenMs,
    CCITJ17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    pub bitrate: u32,
    pub sampling_rate: u32,
    crc: Option<u16>,
    pub frame_bytes: u32,
    private_bit: bool,
    channel_mode: ChannelMode,
    intensity_stereo: bool,
    ms_stereo: bool,
    copyright: bool,
    original: bool,
    emphasis: Emphasis,
}

pub fn read(bytes: &[u8]) -> Result<FrameHeader, DecodingError> {
    let frame_header = if bytes.len() < 4 {
        return Err(DecodingError::UnexpectedEndOfStream);
    } else {
        // Big Endian: the spec considers the first byte to be the 32-24
        u32::from_be_bytes(
            bytes[0..4]
                .try_into()
                .map_err(|_| DecodingError::UnexpectedEndOfStream)?,
        )
    };

    let a = frame_header >> 21;
    let b = frame_header >> 19 & 0b11;
    let c = frame_header >> 17 & 0b11;
    let d = frame_header >> 16 & 0b1;
    let e = frame_header >> 12 & 0b1111;
    let f = frame_header >> 10 & 0b11;
    let g = frame_header >> 9 & 0b1;
    let h = frame_header >> 8 & 0b1;
    let i = frame_header >> 6 & 0b11;
    let j = frame_header >> 4 & 0b11;
    let k = frame_header >> 3 & 0b1;
    let l = frame_header >> 2 & 0b1;
    let m = frame_header & 0b11;

    if a != 0b111_1111_1111 {
        return Err(DecodingError::InvalidFrameHeader);
    }

    if b != 0b11 {
        return Err(DecodingError::UnsupportedVersion);
    }

    if c != 0b01 {
        return Err(DecodingError::UnsupportedLayer);
    }

    let crc = if d == 1 {
        None
    } else {
        if bytes.len() < 6 {
            return Err(DecodingError::UnexpectedEndOfStream);
        } else {
            Some(u16::from_le_bytes(
                bytes[4..6]
                    .try_into()
                    .map_err(|_| DecodingError::UnexpectedEndOfStream)?,
            ))
        }
    };

    // for MPEG-1, Layer III
    let bitrate = match e {
        0b0000 => return Err(DecodingError::UnsupportedBitrate),
        0b0001 => 32,
        0b0010 => 40,
        0b0011 => 48,
        0b0100 => 56,
        0b0101 => 64,
        0b0110 => 80,
        0b0111 => 96,
        0b1000 => 112,
        0b1001 => 128,
        0b1010 => 160,
        0b1011 => 192,
        0b1100 => 224,
        0b1101 => 256,
        0b1110 => 320,
        0b1111 => return Err(DecodingError::UnsupportedBitrate),
        _ => unreachable!(),
    };

    // for MPEG-1, Layer III
    let sampling_rate = match f {
        0b00 => 44100,
        0b01 => 48000,
        0b10 => 32000,
        0b11 => return Err(DecodingError::UnsupportedSamplingRate),
        _ => unreachable!(),
    };

    let padding = g;

    // For Layer III
    let frame_bytes = 144 * bitrate * 1000 / sampling_rate + padding;

    let private_bit = h == 1;

    let channel_mode = match i {
        0b00 => ChannelMode::Stereo,
        0b01 => ChannelMode::JointStereo,
        0b10 => ChannelMode::DualChannel,
        0b11 => ChannelMode::Mono,
        _ => unreachable!(),
    };

    let (intensity_stereo, ms_stereo) = if channel_mode == ChannelMode::JointStereo {
        match j {
            0b00 => (true, false),
            0b01 => (false, true),
            0b10 => (false, false),
            0b11 => (false, false),
            _ => unreachable!(),
        }
    } else {
        (false, false)
    };

    let copyright = k == 1;

    let original = l == 1;

    let emphasis = match m {
        0b00 => Emphasis::None,
        0b01 => Emphasis::FiftyFifteenMs,
        0b10 => return Err(DecodingError::UnsupportedEmphasis),
        0b11 => Emphasis::CCITJ17,
        _ => unreachable!(),
    };

    Ok(FrameHeader {
        bitrate,
        sampling_rate,
        frame_bytes,
        crc,
        private_bit,
        channel_mode,
        intensity_stereo,
        ms_stereo,
        copyright,
        original,
        emphasis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reading_frame_header() {
        let result = read(b"\xFF\xFB\x90\xC4").unwrap();
        assert_eq!(
            result,
            FrameHeader {
                bitrate: 128,
                sampling_rate: 44100,
                crc: None,
                frame_bytes: 417,
                private_bit: false,
                channel_mode: ChannelMode::Mono,
                intensity_stereo: false,
                ms_stereo: false,
                copyright: false,
                original: true,
                emphasis: Emphasis::None,
            }
        );

        let result = read(b"\xFF\xFB\xC0\xC4").unwrap();
        assert_eq!(
            result,
            FrameHeader {
                bitrate: 224,
                sampling_rate: 44100,
                crc: None,
                frame_bytes: 731,
                private_bit: false,
                channel_mode: ChannelMode::Mono,
                intensity_stereo: false,
                ms_stereo: false,
                copyright: false,
                original: true,
                emphasis: Emphasis::None,
            }
        );

        let result = read(b"\xFF\xFB\x30\xC4").unwrap();
        assert_eq!(
            result,
            FrameHeader {
                bitrate: 48,
                sampling_rate: 44100,
                crc: None,
                frame_bytes: 156,
                private_bit: false,
                channel_mode: ChannelMode::Mono,
                intensity_stereo: false,
                ms_stereo: false,
                copyright: false,
                original: true,
                emphasis: Emphasis::None,
            }
        );

        let result = read(b"\xFF\xFB\x20\xC4").unwrap();
        assert_eq!(
            result,
            FrameHeader {
                bitrate: 40,
                sampling_rate: 44100,
                crc: None,
                frame_bytes: 130,
                private_bit: false,
                channel_mode: ChannelMode::Mono,
                intensity_stereo: false,
                ms_stereo: false,
                copyright: false,
                original: true,
                emphasis: Emphasis::None,
            }
        );

        let result = read(b"\xFF\xFB\x10\xC4").unwrap();
        assert_eq!(
            result,
            FrameHeader {
                bitrate: 32,
                sampling_rate: 44100,
                crc: None,
                frame_bytes: 104,
                private_bit: false,
                channel_mode: ChannelMode::Mono,
                intensity_stereo: false,
                ms_stereo: false,
                copyright: false,
                original: true,
                emphasis: Emphasis::None,
            }
        );
    }
}

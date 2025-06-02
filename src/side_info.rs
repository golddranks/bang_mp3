use std::u64;

use crate::{
    DecodingError,
    header::{ChannelMode, FrameHeader},
    read_bits, read_u32, read_u64,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Normal {
        table_select: [u8; 3],
        region0_count: u8,
        region1_count: u8,
    },
    Abnormal {
        block_type: u8,
        mixed_block_flag: bool,
        table_select: [u8; 2],
        subblock_gain: [u8; 3],
    },
}

impl Block {
    fn read_normal(data: u64) -> Result<Self, DecodingError> {
        todo!();
        Ok(Block::Normal {
            table_select: [0, 0, 0],
            region0_count: 0,
            region1_count: 0,
        })
    }

    fn read_abnormal(data: u64) -> Result<Self, DecodingError> {
        let block_type = read_bits(data, 34..36) as u8;
        if block_type == 0 {
            return Err(DecodingError::InvalidBlockType);
        }
        let mixed_block_flag = read_bits(data, 36..37) == 1;
        let region0_table = read_bits(data, 37..42) as u8;
        let region1_table = read_bits(data, 42..47) as u8;
        let table_select = [region0_table, region1_table];
        let subblock0_gain = read_bits(data, 47..50) as u8;
        let subblock1_gain = read_bits(data, 50..53) as u8;
        let subblock2_gain = read_bits(data, 53..56) as u8;
        let subblock_gain = [subblock0_gain, subblock1_gain, subblock2_gain];
        Ok(Block::Abnormal {
            block_type,
            mixed_block_flag,
            table_select,
            subblock_gain,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Granule {
    part2_3_len: u16,
    big_values: u16,
    global_gain: u8,
    scalefac_compress: u8,
    window_switching: bool,
    block: Block,
    preflag: bool,
    scalefac_scale: bool,
    count1table_select: bool,
}

impl Granule {
    pub fn read(data: u64) -> Result<Self, DecodingError> {
        let part2_3_len = read_bits(data, 0..12) as u16;
        let big_values = read_bits(data, 12..21) as u16;
        let global_gain = read_bits(data, 21..29) as u8;
        let scalefac_compress = read_bits(data, 29..33) as u8;
        let window_switching = read_bits(data, 33..34) == 1;

        let block = if window_switching {
            Block::read_abnormal(data)?
        } else {
            Block::read_normal(data)?
        };
        let preflag = read_bits(data, 56..57) == 1;
        let scalefac_scale = read_bits(data, 57..58) == 1;
        let count1table_select = read_bits(data, 58..59) == 1;

        Ok(Granule {
            part2_3_len,
            big_values,
            global_gain,
            scalefac_compress,
            window_switching,
            preflag,
            scalefac_scale,
            count1table_select,
            block,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideInfo {
    main_data_begin: u16,
    private_bits: u8,
    share: u8,
    granule0: Granule,
    granule1: Granule,
}

impl SideInfo {
    pub fn len(header: &FrameHeader) -> usize {
        if header.channel_mode == ChannelMode::Mono {
            17
        } else {
            32
        }
    }

    fn read_mono(side_info_bytes: &[u8]) -> Result<Self, DecodingError> {
        let mut common_bytes = &side_info_bytes[..4];
        let common = read_u32(&mut common_bytes)?;

        let main_data_begin = read_bits(common, 0..9) as u16;
        let private_bits = read_bits(common, 9..14) as u8;
        let share = read_bits(common, 14..18) as u8;

        let mut granule0_bytes = &side_info_bytes[2..10];
        let mut granule1_bytes = &side_info_bytes[9..17];

        let granule0 = (read_u64(&mut granule0_bytes)? << 2) & (u64::MAX << 5);
        let granule1 = read_u64(&mut granule1_bytes)? << 5;

        Ok(SideInfo {
            main_data_begin,
            private_bits,
            share,
            granule0: Granule::read(granule0)?,
            granule1: Granule::read(granule1)?,
        })
    }

    pub fn read(header: &FrameHeader, frame_data: &[u8]) -> Result<Self, DecodingError> {
        let offset = SideInfo::len(header);
        let side_info_bytes = &frame_data[..offset];
        match header.channel_mode {
            ChannelMode::Mono => Self::read_mono(side_info_bytes),
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{FirstFrame, FrameIter};

    use super::*;
    use std::fs::read;

    #[test]
    fn test_real_side_info() {
        let data = read("tests/sine_320hz_50ms_vbr_frame1-3.mp3").unwrap();
        let (first, iter) = FrameIter::new(&data).unwrap();
        let FirstFrame::Cbr(_) = first else {
            panic!("Expected CBR frame")
        };
        let _frames: Vec<_> = iter.map(|frame| frame.unwrap()).collect();
    }

    #[test]
    fn test_mono_side_info() {
        let header = FrameHeader::read(b"\xFF\xFB\x10\xC4").unwrap();
        //                                    <-     common     ->      <- granule0...
        let mono_common = u32::to_be_bytes(0b_000000000_00000_0000______00000000000000);
        //common->   <-           granule0: Spans 59 bits: 18..77.                                ->     <- granule1...
        let mono_granule0 = u64::to_be_bytes(
            0b00_____001100010010_000010000_10101010_1010_1___01_0__11000_00000__000_000_000___1_0_0_____000);
        //granule0->    <-        granule1: Spans 59 bits: 77..136.                                  ->
        let mono_granule1 = u64::to_be_bytes(
            0b00000_____010000110110_001010101_10100110_1111_1___10_0__11110_10000__000_010_000___0_0_0);

        let mut mono_side_info = [0; 17];
        mono_side_info[0..4].copy_from_slice(&mono_common);
        mono_side_info[2..10].copy_from_slice(&mono_granule0);
        mono_side_info[9..17].copy_from_slice(&mono_granule1);

        // OR the overlapping bits
        mono_side_info[2..4].copy_from_slice(&[
            mono_common[2] | mono_granule0[0],
            mono_common[3] | mono_granule0[1],
        ]);
        mono_side_info[9..10].copy_from_slice(&[mono_granule0[7] | mono_granule1[0]]);

        assert_eq!(
            &mono_side_info,
            b"\x00\x00\x0C\x48\x21\x55\x55\x80\x00\x22\x1B\x15\x69\xBF\x3D\x00\x80"
        );

        let side_info = SideInfo::read(&header, &mono_side_info).unwrap();

        assert_eq!(
            side_info,
            SideInfo {
                main_data_begin: 0,
                private_bits: 0,
                share: 0,
                granule0: Granule {
                    part2_3_len: 0b1100010010,
                    big_values: 0b10000,
                    global_gain: 0b10101010,
                    scalefac_compress: 0b1010,
                    window_switching: true,
                    block: Block::Abnormal {
                        block_type: 1,
                        mixed_block_flag: false,
                        table_select: [0b11000, 0],
                        subblock_gain: [0, 0, 0]
                    },
                    preflag: true,
                    scalefac_scale: false,
                    count1table_select: false,
                },
                granule1: Granule {
                    part2_3_len: 0b10000110110,
                    big_values: 0b1010101,
                    global_gain: 0b10100110,
                    scalefac_compress: 0b1111,
                    window_switching: true,
                    block: Block::Abnormal {
                        block_type: 2,
                        mixed_block_flag: false,
                        table_select: [0b11110, 0b10000],
                        subblock_gain: [0, 0b10, 0]
                    },
                    preflag: false,
                    scalefac_scale: false,
                    count1table_select: false,
                }
            }
        );
    }
}

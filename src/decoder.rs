pub fn play_sound(sound: &str) {
    println!("Playing sound: {}", sound);
}

#[cfg(test)]
mod tests {
    use std::fs::read;

    #[test]
    fn test_decoding() {
        let frame_bytes = read("tests/sine_320hz_50ms_vbr_frame1-3.mp3").unwrap();
        let frame = crate::Frame::read(&frame_bytes).unwrap();
        dbg!(frame.side_info);
        dbg!(frame.main_data);
    }
}

pub fn play_sound(sound: &str) {
    println!("Playing sound: {}", sound);
}

#[cfg(test)]
mod tests {
    use std::fs::read;

    #[test]
    fn test_decoding() {
        let frame_bytes = read("test/sine_320hz_50ms_frame0.mp3").unwrap();
        let frame = crate::Frame::read(&frame_bytes).unwrap();
        frame.data;
    }
}

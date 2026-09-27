//! Aether no_std MP3 decoder wrapper.
//!
//! Decoder core is vendored from robbie01/nanomp3 (Apache-2.0/MIT),
//! which is a pure-Rust no_std port based on lieff/minimp3 (CC0).
#![allow(dead_code)]

mod minimp3;

pub const MAX_SAMPLES_PER_FRAME: usize = 1152 * 2;

#[derive(Clone, Copy, PartialEq)]
pub enum Channels {
    Mono = 1,
    Stereo = 2,
}

impl Channels {
    pub fn num(self) -> usize { self as usize }
}

#[derive(Clone, Copy)]
pub struct FrameInfo {
    pub samples_produced: usize,
    pub channels: Channels,
    pub sample_rate: u32,
    pub bitrate: u32,
}

pub struct Decoder(minimp3::mp3dec_t);

impl Decoder {
    pub const fn new() -> Self {
        Self(minimp3::mp3dec_t::new())
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn decode(&mut self, mp3: &[u8], pcm: &mut [f32; MAX_SAMPLES_PER_FRAME])
        -> (usize, Option<FrameInfo>)
    {
        let mut info = minimp3::mp3dec_frame_info_t::default();
        let samples = unsafe {
            minimp3::mp3dec_decode_frame(&mut self.0, mp3, pcm, &mut info)
        };
        if info.frame_bytes == 0 {
            return (0, None);
        }
        let frame_bytes = info.frame_bytes;
        if samples <= 0 {
            return (frame_bytes, None);
        }
        let channels = match info.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => return (frame_bytes, None),
        };
        Some(FrameInfo {
            samples_produced: samples as usize,
            channels,
            sample_rate: info.hz as u32,
            bitrate: info.bitrate_kbps as u32,
        }).map(|f| (frame_bytes, Some(f))).unwrap()
    }
}

impl Default for Decoder {
    fn default() -> Self { Self::new() }
}

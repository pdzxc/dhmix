//! WAV recording of a bus on a background thread, fed by the engine's record tap.

use crate::{CHANNELS, SAMPLE_RATE};
use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Sender};
use std::path::Path;
use std::thread::JoinHandle;

pub struct Recording {
    tx: Option<Sender<Vec<f32>>>,
    thread: Option<JoinHandle<Result<()>>>,
}

impl Recording {
    /// Starts writing 32-bit float WAV to `path`. Returns the sender the engine tap should use.
    pub fn start(path: &Path) -> Result<(Self, Sender<Vec<f32>>)> {
        let spec = hound::WavSpec {
            channels: CHANNELS as u16,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).with_context(|| format!("create {}", path.display()))?;
        let (tx, rx) = bounded::<Vec<f32>>(256);
        let thread = std::thread::Builder::new()
            .name("dhmix-recorder".into())
            .spawn(move || {
                for block in rx {
                    for v in block {
                        writer.write_sample(v)?;
                    }
                }
                writer.finalize()?;
                Ok(())
            })
            .context("spawn recorder thread")?;
        Ok((Self { tx: Some(tx.clone()), thread: Some(thread) }, tx))
    }

    /// Closes the file. Safe to call once; `Drop` does the same.
    pub fn stop(mut self) -> Result<()> {
        self.finish()
    }

    fn finish(&mut self) -> Result<()> {
        self.tx.take();
        match self.thread.take() {
            Some(t) => t.join().map_err(|_| anyhow::anyhow!("recorder thread panicked"))?,
            None => Ok(()),
        }
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_writes_a_readable_wav() {
        let dir = std::env::temp_dir().join(format!("dhmix-rec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wav");
        let (rec, tx) = Recording::start(&path).unwrap();
        tx.send(vec![0.5, -0.5, 0.25, -0.25]).unwrap();
        drop(tx);
        rec.stop().unwrap();
        let mut reader = hound::WavReader::open(&path).unwrap();
        let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap()).collect();
        assert_eq!(samples, vec![0.5, -0.5, 0.25, -0.25]);
        assert_eq!(reader.spec().sample_rate, SAMPLE_RATE);
        let _ = std::fs::remove_dir_all(dir);
    }
}

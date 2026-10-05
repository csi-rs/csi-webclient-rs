//! Local CSI stream export.
//!
//! Decodes raw WebSocket CSI frames (the device's serialized records, as `csi-webserver-rs`
//! forwards them) and writes them to a Parquet file on the client host. See [`csi`] for the formats
//! decoded and [`parquet_sink`] for the versioned schema.

pub mod csi;
pub mod legacy;
pub mod parquet_sink;

use std::sync::Arc;

use csi::{Decoded, FrameDecoder};
use parquet_sink::{ParquetSink, ParquetSinkError};

use crate::profile::ClientProfile;

/// An active recording of one device's CSI stream to a Parquet file.
///
/// Decodes each incoming frame and appends it to the sink. Lives outside
/// [`crate::state::AppState`] (which is `Clone`) because the underlying file writer is not
/// cloneable.
pub struct Recorder {
    sink: ParquetSink,
    decoder: FrameDecoder,
    /// Frames successfully decoded and written.
    pub frames_written: u64,
    /// Frames that failed to decode (unsupported wire version, truncation, wrong chip).
    pub decode_errors: u64,
    /// Session announcements seen. They anchor the device clock and are not written as rows.
    pub sessions_seen: u64,
}

impl Recorder {
    /// Open a Parquet file at `path` for a stream from the given `chip` string.
    ///
    /// The chip is written to the `chip` column when a frame does not name its own, and selects
    /// the pre-0.12 layout for firmware that predates the wire format. An unrecognised chip only
    /// disables that fallback. `profile` labels the `data_format` column from the numeric
    /// `cur_bb_format` where it can (see [`ClientProfile::label_format`]).
    ///
    /// Returns `Err` if the file cannot be created.
    pub fn start(
        path: &str,
        chip: &str,
        profile: Arc<dyn ClientProfile>,
    ) -> Result<Self, String> {
        let sink = ParquetSink::open(path, chip, profile).map_err(|e| e.to_string())?;
        Ok(Self {
            sink,
            decoder: FrameDecoder::new(legacy::chip_from_str(chip)),
            frames_written: 0,
            decode_errors: 0,
            sessions_seen: 0,
        })
    }

    /// The output file path being written.
    pub fn path(&self) -> &str {
        self.sink.path()
    }

    /// Decode one raw WebSocket frame and append it, stamped with `host_rx_micros`
    /// (UTC microseconds). Decode failures are counted, not propagated, so a
    /// single malformed frame never aborts a recording.
    pub fn record_frame(&mut self, bytes: &[u8], host_rx_micros: i64) -> Result<(), ParquetSinkError> {
        match self.decoder.decode(bytes) {
            Ok(Decoded::Csi(rec)) => {
                self.sink.push(*rec, host_rx_micros)?;
                self.frames_written += 1;
            }
            Ok(Decoded::Session(..)) => self.sessions_seen += 1,
            Err(_) => self.decode_errors += 1,
        }
        Ok(())
    }

    /// Flush remaining rows and finalize the Parquet footer.
    pub fn finish(mut self) -> Result<(), ParquetSinkError> {
        self.sink.finish()
    }
}

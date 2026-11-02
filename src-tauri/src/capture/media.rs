use std::future::IntoFuture;

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionMediaProperties,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::DataReader;
use windows::core::Interface as _;
use windows_future::IAsyncOperation;

fn block<T: windows::core::RuntimeType>(operation: IAsyncOperation<T>) -> windows::core::Result<T> {
    pollster::block_on(operation.into_future())
}

// Thumbnails are small; this only guards against bloating the 1s payload.
const MAX_THUMBNAIL_BYTES: usize = 512 * 1024;

pub struct MediaSampler {
    manager: Option<GlobalSystemMediaTransportControlsSessionManager>,
    last_track: Option<(String, String)>,
    last_thumb: Option<String>,
    last_emitted: Option<String>,
}

pub struct MediaStatus {
    pub playing: bool,
    pub title: String,
    pub artist: String,
    // Three-state so the frontend can cache the art: None = unchanged since
    // the previous sample, Some(None) = cleared, Some(Some(url)) = new art.
    pub thumb: Option<Option<String>>,
}

impl MediaSampler {
    pub fn new() -> Self {
        Self {
            manager: None,
            last_track: None,
            last_thumb: None,
            last_emitted: None,
        }
    }

    fn manager(&mut self) -> Option<&GlobalSystemMediaTransportControlsSessionManager> {
        if self.manager.is_none() {
            self.manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                .and_then(block)
                .ok();
        }
        self.manager.as_ref()
    }

    pub fn sample(&mut self) -> Option<MediaStatus> {
        let session = self.manager()?.GetCurrentSession().ok()?;
        let status = session.GetPlaybackInfo().ok()?.PlaybackStatus().ok()?;
        let playing = status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
        let mut title = String::new();
        let mut artist = String::new();
        let mut thumb: Option<String> = None;
        if let Ok(props) = session.TryGetMediaPropertiesAsync().and_then(block) {
            title = props.Title().map(|t| t.to_string()).unwrap_or_default();
            artist = props.Artist().map(|a| a.to_string()).unwrap_or_default();
            let key = (title.clone(), artist.clone());
            if self.last_track.as_ref() != Some(&key) {
                self.last_track = Some(key);
                self.last_thumb = fetch_thumbnail(&props);
            }
            thumb = self.last_thumb.clone();
        }
        let update = if thumb != self.last_emitted {
            self.last_emitted = thumb.clone();
            Some(thumb)
        } else {
            None
        };
        Some(MediaStatus {
            playing,
            title,
            artist,
            thumb: update,
        })
    }
}

impl Default for MediaSampler {
    fn default() -> Self {
        Self::new()
    }
}

fn fetch_thumbnail(
    props: &GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> Option<String> {
    let stream = props
        .Thumbnail()
        .ok()?
        .OpenReadAsync()
        .and_then(block)
        .ok()?;
    let size = stream.Size().ok()? as usize;
    if size == 0 || size > MAX_THUMBNAIL_BYTES {
        return None;
    }
    let input = stream.GetInputStreamAt(0).ok()?;
    let reader = DataReader::CreateDataReader(&input).ok()?;
    let pending = reader.LoadAsync(size as u32).ok()?;
    let loaded: IAsyncOperation<u32> = pending.cast().ok()?;
    block(loaded).ok()?;
    let mut bytes = vec![0u8; size];
    reader.ReadBytes(&mut bytes).ok()?;
    let mime = sniff_mime(&bytes)?;
    Some(data_url(mime, &bytes))
}

fn sniff_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.starts_with(b"GIF8") {
        Some("image/gif")
    } else if bytes.starts_with(b"BM") {
        Some("image/bmp")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn data_url(mime: &str, bytes: &[u8]) -> String {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{mime};base64,{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_image_types() {
        assert_eq!(sniff_mime(&[0xFF, 0xD8, 0xFF, 0x00]), Some("image/jpeg"));
        assert_eq!(
            sniff_mime(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            Some("image/png")
        );
        assert_eq!(sniff_mime(b"GIF89a"), Some("image/gif"));
        assert_eq!(sniff_mime(b"BM6"), Some("image/bmp"));
        assert_eq!(sniff_mime(b"RIFF\x00\x00\x00\x00WEBP"), Some("image/webp"));
        assert_eq!(sniff_mime(b"RIFF\x00\x00\x00\x00AVI "), None);
        assert_eq!(sniff_mime(b"nope"), None);
        assert_eq!(sniff_mime(&[]), None);
    }

    #[test]
    fn builds_data_url() {
        assert_eq!(
            data_url("image/png", &[1, 2, 3]),
            "data:image/png;base64,AQID"
        );
    }
}

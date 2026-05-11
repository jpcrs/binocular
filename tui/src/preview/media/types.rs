#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaKind {
    Audio,
    Video,
}

pub struct MediaPreviewPayload {
    pub text: ratatui::text::Text<'static>,
    pub artwork_bytes: Option<Vec<u8>>,
}

#[derive(Default)]
pub(crate) struct MediaMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub composer: Option<String>,
    pub track: Option<String>,
    pub year: Option<String>,
    pub duration: Option<String>,
    pub bitrate: Option<String>,
    pub sample_rate: Option<String>,
    pub channels: Option<String>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub frame_rate: Option<String>,
    pub artwork: Option<ArtworkInfo>,
}

impl MediaMetadata {
    pub fn merge(&mut self, other: MediaMetadata) {
        merge_opt(&mut self.title, other.title);
        merge_opt(&mut self.artist, other.artist);
        merge_opt(&mut self.album, other.album);
        merge_opt(&mut self.genre, other.genre);
        merge_opt(&mut self.composer, other.composer);
        merge_opt(&mut self.track, other.track);
        merge_opt(&mut self.year, other.year);
        merge_opt(&mut self.duration, other.duration);
        merge_opt(&mut self.bitrate, other.bitrate);
        merge_opt(&mut self.sample_rate, other.sample_rate);
        merge_opt(&mut self.channels, other.channels);
        merge_opt(&mut self.resolution, other.resolution);
        merge_opt(&mut self.codec, other.codec);
        merge_opt(&mut self.frame_rate, other.frame_rate);
        if self.artwork.is_none() {
            self.artwork = other.artwork;
        }
    }
}

pub(crate) fn merge_opt(slot: &mut Option<String>, incoming: Option<String>) {
    if slot.is_none() {
        *slot = incoming.filter(|s| !s.trim().is_empty());
    }
}

#[derive(Clone)]
pub(crate) struct ArtworkInfo {
    pub mime: String,
    pub size_bytes: usize,
    pub dimensions: Option<(u32, u32)>,
    pub data: Vec<u8>,
}

pub(crate) fn has_any_metadata(m: &MediaMetadata) -> bool {
    m.title.is_some()
        || m.artist.is_some()
        || m.album.is_some()
        || m.genre.is_some()
        || m.composer.is_some()
        || m.track.is_some()
        || m.year.is_some()
        || m.duration.is_some()
        || m.bitrate.is_some()
        || m.sample_rate.is_some()
        || m.channels.is_some()
        || m.resolution.is_some()
        || m.codec.is_some()
        || m.frame_rate.is_some()
        || m.artwork.is_some()
}

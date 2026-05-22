use id3::frame::{Comment, Content, Frame, Picture, PictureType};
use id3::{Tag, TagLike, Version};
use std::path::Path;

use crate::spotify::types::SpotifyTrack;
use super::cover::CoverResult;

pub fn tag_mp3(
    mp3_path: &Path,
    track: &SpotifyTrack,
    cover: &CoverResult,
) -> Result<(), String> {
    let mut tag = Tag::new();

    tag.set_title(&track.name);
    tag.set_artist(track.artists.join(", "));
    tag.set_album(&track.album);
    tag.set_track(track.track_number as u32);
    tag.set_disc(track.disc_number as u32);

    if let Some(ref date) = track.release_date {
        if let Some(year) = date.split('-').next().and_then(|y| y.parse::<i32>().ok()) {
            tag.set_year(year);
        }
    }

    if let Some(ref isrc) = track.isrc {
        tag.add_frame(Frame::text("TSRC", isrc));
    }

    tag.add_frame(Frame::with_content("COMM", Content::Comment(Comment {
        lang: "eng".to_string(),
        description: String::new(),
        text: "Spytfy".to_string(),
    })));

    tag.add_frame(Frame::with_content("APIC", Content::Picture(Picture {
        mime_type: "image/jpeg".to_string(),
        picture_type: PictureType::CoverFront,
        description: String::new(),
        data: cover.bytes.clone(),
    })));

    tag.write_to_path(mp3_path, Version::Id3v24)
        .map_err(|e| format!("Failed to write ID3 tags: {e}"))?;

    Ok(())
}

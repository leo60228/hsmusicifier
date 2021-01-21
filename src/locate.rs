use crate::bandcamp;
use anyhow::{anyhow, Context, Result};
use std::convert::TryInto;

pub fn find_id3_bandcamp<'a, 'b>(
    tag: &'a id3::Tag,
    albums: &'b [bandcamp::Album],
) -> Result<&'b bandcamp::Track> {
    let album_name = tag.album().context("missing album")?;
    let mut track_num: usize = tag.track().context("missing track")?.try_into()?;

    // frustracean
    if album_name == "Homestuck Vol. 9-10 (with [S] Collide. and Act 7)" && track_num >= 52 {
        track_num -= 1;
    }

    let album = albums
        .iter()
        .find(|x| x.name == album_name)
        .ok_or_else(|| anyhow!("couldn't find album {:?}", album_name))?;
    let track = &album.tracks[track_num - 1];
    Ok(track)
}

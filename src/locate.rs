use crate::{bandcamp, hsmusic};
use anyhow::{anyhow, Context, Result};
use std::convert::TryInto;

pub fn find_bandcamp_from_id3<'a, 'b>(
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

fn find_hsmusic<'a>(
    albums: &'a [hsmusic::Album<'a>],
    mut f: impl FnMut(&'a hsmusic::Album, &'a hsmusic::Track) -> bool,
) -> Option<(&'a hsmusic::Album, &'a hsmusic::Track)> {
    albums
        .iter()
        .flat_map(|album| album.tracks.iter().map(move |track| (album, track)))
        .find(|(album, track)| f(album, track))
}

pub fn find_hsmusic_from_bandcamp<'a, 'b>(
    bandcamp: &'a bandcamp::Track,
    albums: &'b [hsmusic::Album<'b>],
) -> Result<(&'b hsmusic::Album<'b>, &'b hsmusic::Track<'b>)> {
    find_hsmusic(albums, |_, track| {
        track.urls.iter().any(|&x| x == bandcamp.url)
    })
    .ok_or_else(|| anyhow!("couldn't find track {:?}", bandcamp.name))
}

fn special_hsmusic_from_id3<'a, 'b>(
    tag: &'a id3::Tag,
    albums: &'b [hsmusic::Album<'b>],
) -> Result<Option<(&'b hsmusic::Album<'b>, &'b hsmusic::Track<'b>)>> {
    let album_name = tag.album().context("missing album")?;
    let track_num = tag.track().context("missing track")?;

    Ok(match (album_name, track_num) {
        ("Homestuck Vol. 9-10 (with [S] Collide. and Act 7)", 51) => {
            find_hsmusic(albums, |_, track| track.name == "Frustracean")
        }
        ("HIVESWAP: ACT 2 Original Soundtrack", 13) => {
            find_hsmusic(albums, |_, track| track.name == "Objection")
        }
        _ => None,
    })
}

pub fn find_hsmusic_from_id3<'a, 'b, 'c>(
    id3: &'a id3::Tag,
    bandcamp_albums: &'b [bandcamp::Album],
    hsmusic_albums: &'c [hsmusic::Album<'c>],
) -> Result<(&'c hsmusic::Album<'c>, &'c hsmusic::Track<'c>)> {
    if let Some(special) = special_hsmusic_from_id3(id3, hsmusic_albums)? {
        Ok(special)
    } else {
        let bandcamp = find_bandcamp_from_id3(id3, bandcamp_albums)?;
        let hsmusic = find_hsmusic_from_bandcamp(bandcamp, hsmusic_albums)?;
        Ok(hsmusic)
    }
}

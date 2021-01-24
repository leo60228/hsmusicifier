use crate::{bandcamp, hsmusic};
use anyhow::{anyhow, Result};

pub fn find_bandcamp_from_album_track<'a, 'b>(
    album_name: &'a str,
    mut track_num: usize,
    albums: &'b [bandcamp::Album],
) -> Result<Option<&'b bandcamp::Track>> {
    // frustracean
    if album_name == "Homestuck Vol. 9-10 (with [S] Collide. and Act 7)" && track_num >= 52 {
        track_num -= 1;
    }

    if let Some(album) = albums.iter().find(|x| x.name == album_name) {
        let track = &album.tracks[track_num - 1];
        Ok(Some(track))
    } else {
        Ok(None)
    }
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

fn special_hsmusic_from_album_track<'a, 'b>(
    album_name: &'a str,
    track_num: usize,
    albums: &'b [hsmusic::Album<'b>],
) -> Option<(&'b hsmusic::Album<'b>, &'b hsmusic::Track<'b>)> {
    match (album_name, track_num) {
        ("Homestuck Vol. 9-10 (with [S] Collide. and Act 7)", 51) => {
            find_hsmusic(albums, |_, track| track.name == "Frustracean")
        }
        ("HIVESWAP: ACT 2 Original Soundtrack", 13) => {
            find_hsmusic(albums, |_, track| track.name == "Objection")
        }
        _ => None,
    }
}

fn bandcamp_to_hsmusic_name(album: &str) -> &str {
    match album {
        "Homestuck - Strife!" => "Strife!",
        other => other,
    }
}

pub fn find_hsmusic_from_album_track<'a, 'b, 'c>(
    album_name: &'a str,
    track_num: usize,
    bandcamp_albums: &'b [bandcamp::Album],
    hsmusic_albums: &'c [hsmusic::Album<'c>],
) -> Result<(&'c hsmusic::Album<'c>, &'c hsmusic::Track<'c>)> {
    if let Some(special) = special_hsmusic_from_album_track(album_name, track_num, hsmusic_albums) {
        Ok(special)
    } else if let Some(bandcamp) =
        find_bandcamp_from_album_track(album_name, track_num, bandcamp_albums)?
    {
        let hsmusic = find_hsmusic_from_bandcamp(bandcamp, hsmusic_albums)?;
        Ok(hsmusic)
    } else {
        let album = hsmusic_albums
            .iter()
            .find(|x| x.name == bandcamp_to_hsmusic_name(album_name))
            .ok_or_else(|| {
                anyhow!(
                    "couldn't find album {:?} in known albums {:?}",
                    album_name,
                    hsmusic_albums.iter().map(|x| x.name).collect::<Vec<_>>()
                )
            })?;
        let track = &album.tracks[track_num - 1];
        Ok((album, track))
    }
}

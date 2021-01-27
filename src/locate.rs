use crate::{bandcamp, hsmusic};
use anyhow::{anyhow, Result};

pub fn find_bandcamp_from_album_track<'a, 'b>(
    album_name: &'a str,
    title: &'a str,
    track_num: usize,
    albums: &'b [bandcamp::Album],
) -> Result<Option<&'b bandcamp::Track>> {
    if let Some(album) = albums.iter().find(|x| x.name == album_name) {
        let track = album
            .tracks
            .iter()
            .find(|x| x.name == title || x.name.splitn(2, " - ").last().unwrap_or("") == title)
            .or_else(|| album.tracks.get(track_num - 1))
            .ok_or_else(|| {
                anyhow!(
                    "found bandcamp album {:?} but not track {:?}",
                    album_name,
                    title
                )
            })?;
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
    find_hsmusic(albums, |album, track| {
        !matches!(
            album.name,
            "Homestuck Vol. 1" | "Homestuck Vol. 2" | "Homestuck Vol. 3" | "Homestuck Vol. 4"
        ) && track.urls.iter().any(|&x| x == bandcamp.url)
    })
    .ok_or_else(|| anyhow!("couldn't find track {:?}", bandcamp.name))
}

fn special_hsmusic_from_album_track<'a, 'b, 'c>(
    album_name: &'a str,
    title: &'a str,
    track_num: usize,
    bandcamp_albums: &'b [bandcamp::Album],
    hsmusic_albums: &'c [hsmusic::Album<'c>],
) -> Result<Option<(&'c hsmusic::Album<'c>, &'c hsmusic::Track<'c>)>> {
    match (album_name, title) {
        ("Homestuck Vol. 9-10 (with [S] Collide. and Act 7)", "Frustracean") => {
            Ok(find_hsmusic(hsmusic_albums, |_, track| {
                track.name == "Frustracean"
            }))
        }
        ("HIVESWAP: ACT 2 Original Soundtrack", title) => Ok(Some(find_hsmusic_from_album_track(
            "Hiveswap Act 2 OST",
            title,
            track_num,
            bandcamp_albums,
            hsmusic_albums,
        )?)),
        _ => Ok(None),
    }
}

fn bandcamp_to_hsmusic_name(album: &str) -> &str {
    match album {
        "Homestuck - Strife!" => "Strife!",
        other => other.trim_end_matches(" [UNOFFICIAL ALBUM]"),
    }
}

pub fn find_hsmusic_from_album_track<'a, 'b, 'c>(
    album_name: &'a str,
    title: &'a str,
    track_num: usize,
    bandcamp_albums: &'b [bandcamp::Album],
    hsmusic_albums: &'c [hsmusic::Album<'c>],
) -> Result<(&'c hsmusic::Album<'c>, &'c hsmusic::Track<'c>)> {
    if let Some(special) = special_hsmusic_from_album_track(
        album_name,
        title,
        track_num,
        bandcamp_albums,
        hsmusic_albums,
    )? {
        Ok(special)
    } else if let Some(album) = hsmusic_albums
        .iter()
        .find(|x| x.name == bandcamp_to_hsmusic_name(album_name))
    {
        let track = &album
            .tracks
            .iter()
            .find(|x| x.name == title)
            .or_else(|| album.tracks.get(track_num - 1))
            .ok_or_else(|| anyhow!("couldn't find track {:?} in album {:?}", title, album.name))?;
        Ok((album, track))
    } else {
        let bandcamp =
            find_bandcamp_from_album_track(album_name, title, track_num, bandcamp_albums)?
                .ok_or_else(|| anyhow!("couldn't find track {:?}", title))?;
        let hsmusic = find_hsmusic_from_bandcamp(bandcamp, hsmusic_albums)?;
        Ok(hsmusic)
    }
}

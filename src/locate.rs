use crate::{bandcamp, hsmusic};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::convert::TryInto;

pub type BandcampLookup<'a> = HashMap<(&'a str, usize), &'a bandcamp::Track>;

pub fn build_bandcamp_lookup<'a>(albums: &'a [bandcamp::Album]) -> BandcampLookup<'a> {
    albums
        .iter()
        .flat_map(|x| x.tracks.iter().enumerate().map(move |y| (&*x.name, y)))
        .map(|(x, (i, y))| ((x, i), y))
        .collect()
}

fn lookup_bandcamp_from_album_track<'a, 'b, 'c>(
    album_name: &'a str,
    track_num: u32,
    lookup: &'b BandcampLookup<'c>,
) -> Result<&'c bandcamp::Track> {
    let mut track_num: usize = track_num.try_into()?;

    // frustracean
    if album_name == "Homestuck Vol. 9-10 (with [S] Collide. and Act 7)" && track_num >= 52 {
        track_num -= 1;
    }

    let track = lookup
        .get(&(album_name, track_num - 1))
        .ok_or_else(|| anyhow!("couldn't find album {:?}", album_name))?;
    Ok(track)
}

pub type HsmusicLookup<'a, 'b> = HashMap<&'a str, (&'b hsmusic::Album<'b>, &'b hsmusic::Track<'b>)>;

fn find_hsmusic<'a>(
    albums: &'a [hsmusic::Album<'a>],
    mut f: impl FnMut(&'a hsmusic::Album, &'a hsmusic::Track) -> bool,
) -> Option<(&'a hsmusic::Album, &'a hsmusic::Track)> {
    albums
        .iter()
        .flat_map(|album| album.tracks.iter().map(move |track| (album, track)))
        .find(|(album, track)| f(album, track))
}

fn find_hsmusic_from_bandcamp<'a, 'b>(
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
    track_num: u32,
    albums: &'b [hsmusic::Album<'b>],
) -> Result<Option<(&'b hsmusic::Album<'b>, &'b hsmusic::Track<'b>)>> {
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

pub fn lookup_hsmusic_from_id3<'a, 'b, 'c, 'd, 'e>(
    tag: &'a id3::Tag,
    bandcamp_lookup: &'b BandcampLookup<'c>,
    hsmusic_albums: &'d [hsmusic::Album<'d>],
    hsmusic_lookup: &'e HsmusicLookup<'b, 'd>,
) -> Result<(&'d hsmusic::Album<'d>, &'d hsmusic::Track<'d>)> {
    let album_name = tag.album().context("missing album")?;
    let track_num = tag.track().context("missing track num")?;

    if let Some(special) = special_hsmusic_from_album_track(album_name, track_num, hsmusic_albums)?
    {
        Ok(special)
    } else {
        let bandcamp = lookup_bandcamp_from_album_track(album_name, track_num, bandcamp_lookup)?;
        let &hsmusic = hsmusic_lookup
            .get(&*bandcamp.url)
            .context("couldn't find track")?;
        Ok(hsmusic)
    }
}

pub fn build_hsmusic_lookup<'a, 'b, 'c>(
    bandcamp: &'a BandcampLookup<'b>,
    hsmusic: &'c [hsmusic::Album<'c>],
) -> HsmusicLookup<'b, 'c> {
    bandcamp
        .values()
        .filter_map(|x| Some((&*x.url, find_hsmusic_from_bandcamp(x, hsmusic).ok()?)))
        .collect()
}

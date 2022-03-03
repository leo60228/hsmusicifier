use anyhow::{anyhow, Context, Error, Result};
use locate::*;
use lofty::{ItemKey, Picture, PictureType};
use rayon::prelude::*;
use std::fmt::Write;
use std::fs::{create_dir_all, read_dir, read_to_string, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;
use walkdir::WalkDir;

pub mod bandcamp;
pub mod hsmusic;
pub mod locate;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ArtType {
    AlbumArt,
    TrackArt,
}

impl FromStr for ArtType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "album" | "album-art" => Ok(Self::AlbumArt),
            "track" | "track-art" => Ok(Self::TrackArt),
            _ => Err(anyhow!("Bad art type {}!", s)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ArtTypes {
    pub first: ArtType,
    pub rest: ArtType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Edits {
    pub add_artists: bool,
    pub add_art: Option<ArtTypes>,
    pub add_album: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn add_art(
    bandcamp_json: PathBuf,
    mut hsmusic_data: PathBuf,
    hsmusic_media: PathBuf,
    edits: Edits,
    verbose: bool,
    in_dir: PathBuf,
    out_dir: PathBuf,
    progress: impl Fn(usize) + Send + Sync,
) -> Result<()> {
    let bandcamp_file = File::open(bandcamp_json)?;
    let bandcamp_reader = BufReader::new(bandcamp_file);
    let bandcamp_albums: Vec<bandcamp::Album> = serde_json::from_reader(bandcamp_reader)?;

    hsmusic_data.push("album");

    let hsmusic_album_texts: Vec<_> = read_dir(hsmusic_data)?
        .map(|ent| {
            let ent = ent?;
            Ok(read_to_string(ent.path())?)
        })
        .collect::<Result<_>>()?;

    let hsmusic_albums: Vec<_> = hsmusic_album_texts
        .iter()
        .map(|x| hsmusic::parse_album(x))
        .collect::<Result<_>>()?;

    let entries: Vec<_> = WalkDir::new(&in_dir)
        .into_iter()
        .filter(|x| {
            if let Ok(x) = x {
                x.file_type().is_file()
            } else {
                true
            }
        })
        .collect::<std::result::Result<_, _>>()?;
    let entries_count = entries.len();
    let mut errors: Vec<_> = entries
        .into_par_iter()
        .map(|entry| {
            progress(entries_count);

            let in_path = entry.path();
            let rel_path = in_path.strip_prefix(&in_dir)?;
            let out_path = out_dir.join(&rel_path);

            if let Some(parent) = out_path.parent() {
                create_dir_all(parent)?;
            }

            println!("{:?} -> {:?}", in_path, out_path);

            std::fs::copy(&in_path, &out_path)?;

            if let Ok(mut metadata) = lofty::read_from_path(&in_path, false) {
                let add_art = edits.add_art.is_some();

                if let Some(tag) = metadata.primary_tag_mut() {
                    if let (Some(album_name), Some(track_num), Some(title)) = (
                        tag.get_string(&ItemKey::AlbumTitle),
                        tag.get_string(&ItemKey::TrackNumber),
                        tag.get_string(&ItemKey::TrackTitle),
                    ) {
                        let track_num = track_num.parse()?;

                        let (album, track) = find_hsmusic_from_album_track(
                            album_name,
                            title,
                            track_num,
                            &bandcamp_albums,
                            &hsmusic_albums,
                        )
                        .with_context(|| {
                            format!("failed to find hsmusic track for {:?}", in_path)
                        })?;

                        if verbose {
                            println!("hsmusic: {:?} - {:?}", album.name, track.name);
                        }

                        if add_art {
                            if let Some(ArtTypes { first, rest }) = edits.add_art {
                                let track_num = if edits.add_album {
                                    track.track_num
                                } else {
                                    track_num
                                };
                                let art = if track_num <= 1 { first } else { rest };
                                let path = track.picture(album, &hsmusic_media, art)?;

                                let mut picture = Picture::from_reader(
                                    &mut File::open(path).context("failed to open picture")?,
                                )
                                .context("failed to create Picture")?;
                                picture.set_pic_type(PictureType::CoverFront);

                                tag.remove_picture_type(PictureType::CoverFront);
                                tag.push_picture(picture);
                            }
                        }

                        if edits.add_artists {
                            if let Some(artists) = &track.artists {
                                let artists =
                                    artists.iter().map(|x| x.who).collect::<Vec<_>>().join(", ");

                                if verbose {
                                    println!("artists: {}", artists);
                                }

                                tag.insert_text(ItemKey::TrackArtist, artists);
                            }
                        }

                        if edits.add_album {
                            tag.insert_text(ItemKey::AlbumTitle, album.name.to_string());
                            tag.insert_text(ItemKey::TrackNumber, track.track_num.to_string());
                            tag.insert_text(
                                ItemKey::RecordingDate,
                                album.date.format("%F").to_string(),
                            );
                        }
                    }
                }

                metadata
                    .save_to_path(&out_path)
                    .context("failed to write metadata")?;
            } else if verbose {
                println!("not audio");
            }

            Ok(())
        })
        .filter_map(|x: Result<(), anyhow::Error>| x.err())
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        let mut msgs = "Errors:\n".to_string();
        for error in errors.drain(..5.min(errors.len())) {
            writeln!(msgs, "* {:?}", error)?;
        }
        if !errors.is_empty() {
            writeln!(msgs, "* ...and {} more", errors.len())?;
        }
        Err(anyhow!("{}", msgs))
    }
}

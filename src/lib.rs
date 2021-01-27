use anyhow::{anyhow, Context, Error, Result};
use ffmpeg_next::*;
use locate::*;
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

fn get_album(metadata: &DictionaryRef) -> Option<(&'static str, String)> {
    metadata
        .get("album")
        .map(|x| ("album", x))
        .or_else(|| metadata.get("ALBUM").map(|x| ("ALBUM", x)))
        .map(|(k, v)| (k, v.to_string()))
}

fn get_track(metadata: &DictionaryRef) -> Result<Option<(&'static str, usize)>> {
    metadata
        .get("track")
        .map(|x| ("track", x))
        .or_else(|| metadata.get("TRACK").map(|x| ("TRACK", x)))
        .map(|(k, v)| Ok((k, v.parse()?)))
        .transpose()
}

fn get_title(metadata: &DictionaryRef) -> Option<(&'static str, String)> {
    metadata
        .get("title")
        .map(|x| ("title", x))
        .or_else(|| metadata.get("TITLE").map(|x| ("TITLE", x)))
        .map(|(k, v)| (k, v.to_string()))
}

pub fn add_art(
    bandcamp_json: PathBuf,
    hsmusic: PathBuf,
    edits: Edits,
    verbose: bool,
    in_dir: PathBuf,
    out_dir: PathBuf,
    progress: impl Fn(usize) + Send + Sync,
) -> Result<()> {
    ffmpeg_next::init()?;

    let bandcamp_file = File::open(bandcamp_json)?;
    let bandcamp_reader = BufReader::new(bandcamp_file);
    let bandcamp_albums: Vec<bandcamp::Album> = serde_json::from_reader(bandcamp_reader)?;

    let hsmusic_albums_path = {
        let mut p = hsmusic.clone();
        p.push("data");
        p.push("album");
        p
    };

    let hsmusic_album_texts: Vec<_> = read_dir(hsmusic_albums_path)?
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

            if let Ok(mut ictx) = ffmpeg_next::format::input(&in_path) {
                if ictx.format().name() == "swf"
                    || !ictx
                        .streams()
                        .any(|x| x.codec().medium() == media::Type::Audio)
                {
                    if verbose {
                        println!("not audio");
                    }
                    std::fs::copy(in_path, out_path)?;
                    return Ok(());
                }

                let mut file_metadata = ictx.metadata().to_owned();

                let add_art = edits.add_art.is_some() && ictx.format().name() != "ogg";

                let mut octx = ffmpeg_next::format::output(&out_path)?;

                let mut stream_mapping = vec![0; ictx.nb_streams() as _];
                let mut ist_time_bases = vec![Rational(0, 1); ictx.nb_streams() as _];
                let mut ost_index = 0;
                let mut picture = None;
                for (ist_index, ist) in ictx.streams().enumerate() {
                    let ist_medium = ist.codec().medium();
                    if ist_medium != media::Type::Audio
                        && ist_medium != media::Type::Subtitle
                        && add_art
                    {
                        stream_mapping[ist_index] = -1;
                        continue;
                    }

                    stream_mapping[ist_index] = ost_index;
                    ist_time_bases[ist_index] = ist.time_base();
                    ost_index += 1;
                    let mut ost = octx.add_stream(encoder::find(codec::Id::None))?;
                    ost.set_parameters(ist.parameters());

                    let mut track_metadata = ist.metadata().to_owned();

                    if ist_medium == media::Type::Audio {
                        let album_track = if let (Some(album), Some(track), Some(title)) = (
                            get_album(&file_metadata),
                            get_track(&file_metadata)?,
                            get_title(&file_metadata),
                        ) {
                            Some((album, track, title, &mut file_metadata))
                        } else if let (Some(album), Some(track), Some(title)) = (
                            get_album(&track_metadata),
                            get_track(&track_metadata)?,
                            get_title(&track_metadata),
                        ) {
                            Some((album, track, title, &mut track_metadata))
                        } else {
                            None
                        };

                        if let Some((
                            (album_key, album_name),
                            (track_key, track_num),
                            (_, title),
                            metadata_dict,
                        )) = album_track
                        {
                            let (album, track) = find_hsmusic_from_album_track(
                                &album_name,
                                &title,
                                track_num,
                                &bandcamp_albums,
                                &hsmusic_albums,
                            )?;

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
                                    picture = Some(track.picture(&album, &hsmusic, art)?);
                                }
                            }

                            if edits.add_artists {
                                let artist_key = if metadata_dict.get("ARTIST").is_some() {
                                    "ARTIST"
                                } else {
                                    "artist"
                                };

                                if let Some(artists) = &track.artists {
                                    let artists = artists
                                        .iter()
                                        .map(|x| x.who)
                                        .collect::<Vec<_>>()
                                        .join(", ");

                                    if verbose {
                                        println!("artists: {}", artists);
                                    }

                                    metadata_dict.set(artist_key, &artists);
                                }
                            }

                            if edits.add_album {
                                let date_key = if metadata_dict.get("DATE").is_some() {
                                    "DATE"
                                } else {
                                    "date"
                                };

                                metadata_dict.set(album_key, &album.name);
                                metadata_dict.set(track_key, &track.track_num.to_string());
                                metadata_dict.set(date_key, &album.date.format("%F").to_string());
                            }
                        }
                    }

                    ost.set_metadata(track_metadata);

                    // We need to set codec_tag to 0 lest we run into incompatible codec tag
                    // issues when muxing into a different container format. Unfortunately
                    // there's no high level API to do this (yet).
                    unsafe {
                        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
                    }
                }

                if add_art {
                    let mut pctx =
                        ffmpeg_next::format::input(&picture.context("couldn't find metadata")?)?;
                    let pst = pctx.streams().next().context("couldn't read art")?;
                    let mut opst = octx.add_stream(encoder::find(codec::Id::None))?;
                    opst.set_parameters(pst.parameters());
                    unsafe {
                        (*opst.parameters().as_mut_ptr()).codec_tag = 0;
                    }

                    let mut picture_metadata = Dictionary::new();
                    picture_metadata.set("title", "cover");
                    picture_metadata.set("comment", "Cover (front)");
                    opst.set_metadata(picture_metadata);

                    unsafe {
                        (*opst.as_mut_ptr()).disposition =
                            format::stream::disposition::Disposition::ATTACHED_PIC.bits();
                    }

                    let pst_time_base = pst.time_base();

                    octx.set_metadata(file_metadata);
                    octx.write_header()?;

                    for (stream, mut packet) in ictx.packets() {
                        let ist_index = stream.index();
                        let ost_index = stream_mapping[ist_index];
                        if ost_index < 0 {
                            continue;
                        }
                        let ost = octx.stream(ost_index as _).unwrap();
                        packet.rescale_ts(ist_time_bases[ist_index], ost.time_base());
                        packet.set_position(-1);
                        packet.set_stream(ost_index as _);
                        packet.write_interleaved(&mut octx)?;
                    }

                    for (_, mut packet) in pctx.packets() {
                        let ost = octx.stream(ost_index as _).unwrap();
                        packet.rescale_ts(pst_time_base, ost.time_base());
                        packet.set_position(-1);
                        packet.set_stream(ost_index as _);
                        packet.write_interleaved(&mut octx)?;
                    }
                } else {
                    octx.set_metadata(file_metadata);
                    octx.write_header()?;

                    for (stream, mut packet) in ictx.packets() {
                        let ist_index = stream.index();
                        let ost_index = stream_mapping[ist_index];
                        if ost_index < 0 {
                            continue;
                        }
                        let ost = octx.stream(ost_index as _).unwrap();
                        packet.rescale_ts(ist_time_bases[ist_index], ost.time_base());
                        packet.set_position(-1);
                        packet.set_stream(ost_index as _);
                        packet.write_interleaved(&mut octx)?;
                    }
                }

                octx.write_trailer()?;
            } else {
                if verbose {
                    println!("not ffmpeg");
                }

                std::fs::copy(in_path, out_path)?;
            }

            Ok(())
        })
        .filter_map(|x: Result<(), anyhow::Error>| x.err())
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        let mut msgs = "Errors:\n".to_string();
        for error in errors.drain(..10.min(errors.len())) {
            writeln!(msgs, "* {}", error)?;
        }
        if !errors.is_empty() {
            writeln!(msgs, "* ...and {} more", errors.len())?;
        }
        Err(anyhow!("{}", msgs))
    }
}

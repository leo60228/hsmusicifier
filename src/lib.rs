use anyhow::{Context, Result};
use ffmpeg_next::*;
use locate::*;
use std::fs::{create_dir_all, read_dir, read_to_string, File};
use std::io::BufReader;
use std::path::PathBuf;
use walkdir::WalkDir;

pub mod bandcamp;
pub mod hsmusic;
pub mod locate;

fn get_album<'a>(metadata: &'a DictionaryRef) -> Option<&'a str> {
    metadata.get("album").or_else(|| metadata.get("ALBUM"))
}

fn get_track(metadata: &DictionaryRef) -> Result<Option<usize>> {
    metadata
        .get("track")
        .or_else(|| metadata.get("TRACK"))
        .map(|x| Ok(x.parse()?))
        .transpose()
}

pub fn add_art(
    bandcamp_json: PathBuf,
    hsmusic: PathBuf,
    verbose: bool,
    in_dir: PathBuf,
    out_dir: PathBuf,
    mut progress: impl FnMut(usize, usize),
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
    for (i, entry) in entries.into_iter().enumerate() {
        progress(i, entries_count);

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
                continue;
            }

            let mut file_metadata = ictx.metadata().to_owned();

            let supports_art = ictx.format().name() != "ogg";

            let mut octx = ffmpeg_next::format::output(&out_path)?;

            let mut stream_mapping = vec![0; ictx.nb_streams() as _];
            let mut ist_time_bases = vec![Rational(0, 1); ictx.nb_streams() as _];
            let mut ost_index = 0;
            let mut picture = None;
            for (ist_index, ist) in ictx.streams().enumerate() {
                let ist_medium = ist.codec().medium();
                if ist_medium != media::Type::Audio && ist_medium != media::Type::Subtitle {
                    stream_mapping[ist_index] = -1;
                    continue;
                }

                stream_mapping[ist_index] = ost_index;
                ist_time_bases[ist_index] = ist.time_base();
                ost_index += 1;
                let mut ost = octx.add_stream(encoder::find(codec::Id::None))?;
                ost.set_parameters(ist.parameters());

                if ist_medium == media::Type::Audio {
                    let mut track_metadata = ist.metadata().to_owned();

                    let album_track = if let (Some(album), Some(track)) =
                        (get_album(&file_metadata), get_track(&file_metadata)?)
                    {
                        Some((album, track))
                    } else if let (Some(album), Some(track)) =
                        (get_album(&track_metadata), get_track(&track_metadata)?)
                    {
                        Some((album, track))
                    } else {
                        None
                    };

                    if let Some((album, track)) = album_track {
                        let (album, track) = find_hsmusic_from_album_track(
                            album,
                            track,
                            &bandcamp_albums,
                            &hsmusic_albums,
                        )?;

                        if verbose {
                            println!("hsmusic: {:?} - {:?}", album.name, track.name);
                        }

                        if supports_art {
                            picture = Some(track.picture(&album, &hsmusic)?);
                        }

                        let (artist_dict, artist_name) = if track_metadata.get("artist").is_some() {
                            (&mut track_metadata, "artist")
                        } else if track_metadata.get("ARTIST").is_some() {
                            (&mut track_metadata, "ARTIST")
                        } else if file_metadata.get("artist").is_some() {
                            (&mut file_metadata, "artist")
                        } else if file_metadata.get("ARTIST").is_some() {
                            (&mut file_metadata, "ARTIST")
                        } else if track_metadata.get("track").is_some()
                            || track_metadata.get("TRACK").is_some()
                        {
                            (&mut track_metadata, "artist")
                        } else {
                            (&mut file_metadata, "artist")
                        };

                        if let Some(artists) = &track.artists {
                            let artists =
                                artists.iter().map(|x| x.who).collect::<Vec<_>>().join(", ");

                            if verbose {
                                println!("artists: {}", artists);
                            }

                            artist_dict.set(artist_name, &artists);
                        }
                    }

                    ost.set_metadata(track_metadata);
                }

                // We need to set codec_tag to 0 lest we run into incompatible codec tag
                // issues when muxing into a different container format. Unfortunately
                // there's no high level API to do this (yet).
                unsafe {
                    (*ost.parameters().as_mut_ptr()).codec_tag = 0;
                }
            }

            if supports_art {
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
    }

    Ok(())
}

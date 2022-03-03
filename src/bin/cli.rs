use anyhow::Result;
use clap::Parser;
use hsmusicifier::{add_art, ArtType, ArtTypes, Edits};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(
    name = "hsmusicifier",
    about = "A tool to add track art to Homestuck music."
)]
struct Opt {
    /// Location of dumped bandcamp json
    #[clap(short, long = "bandcamp-json", parse(from_os_str))]
    pub bandcamp_json: PathBuf,

    /// Location of hsmusic
    #[clap(short = 'm', long, parse(from_os_str))]
    pub hsmusic: PathBuf,

    /// Verbosity
    #[clap(short, long)]
    pub verbose: bool,

    /// Input directory
    #[clap(parse(from_os_str))]
    pub in_dir: PathBuf,

    /// Output directory
    #[clap(parse(from_os_str))]
    pub out_dir: PathBuf,

    /// Don't add art
    #[clap(long)]
    pub no_art: bool,

    /// Use album or track art for first song in album
    #[clap(long, default_value = "album", conflicts_with = "no-art")]
    pub first_art: ArtType,

    /// Use album or track art for remaining songs in album
    #[clap(long, default_value = "track", conflicts_with = "no-art")]
    pub rest_art: ArtType,

    /// Don't add artists
    #[clap(long)]
    pub no_artists: bool,

    /// Add album
    #[clap(long)]
    pub album: bool,
}

fn main() -> Result<()> {
    let opt = Opt::parse();

    let Opt {
        bandcamp_json,
        hsmusic,
        verbose,
        in_dir,
        out_dir,
        no_art,
        first_art,
        rest_art,
        no_artists,
        album,
    } = opt;

    let edits = Edits {
        add_art: if no_art {
            None
        } else {
            Some(ArtTypes {
                first: first_art,
                rest: rest_art,
            })
        },
        add_artists: !no_artists,
        add_album: album,
    };

    add_art(
        bandcamp_json,
        hsmusic,
        edits,
        verbose,
        in_dir,
        out_dir,
        drop,
    )?;

    Ok(())
}

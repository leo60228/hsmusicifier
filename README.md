# hsmusicifier

A tool to add track art from https://hsmusic.wiki to a collection of mp3s.

## Usage (GUI)
Run `hsmusicifier` and pass the data files (see `--help`). The data files can be obtained from releases. Alternatively, `hsmusic` can be obtained from https://notabug.org/hsmusic/hsmusic and `bandcamp.json` can be obtained from `dump_bandcamp`.

## Usage (CLI)
Run `cli`.

```
hsmusicifier 0.3.0
A tool to add track art to Homestuck music.

USAGE:
    cli [FLAGS] [OPTIONS] <in-dir> <out-dir> --bandcamp-json <bandcamp-json> --hsmusic <hsmusic>

FLAGS:
        --album         Add album
    -h, --help          Prints help information
        --no-art        Don't add art
        --no-artists    Don't add artists
    -V, --version       Prints version information
    -v, --verbose       Verbosity

OPTIONS:
    -b, --bandcamp-json <bandcamp-json>    Location of dumped bandcamp json
        --first-art <first-art>            Use album or track art for first song in album [default: album]
    -m, --hsmusic <hsmusic>                Location of hsmusic
        --rest-art <rest-art>              Use album or track art for remaining songs in album [default: track]

ARGS:
    <in-dir>     Input directory
    <out-dir>    Output directory
```

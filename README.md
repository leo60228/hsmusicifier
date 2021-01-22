# hsmusicifier

A tool to add track art from https://hsmusic.wiki to a collection of mp3s.

## Usage (GUI)
Run `hsmusicifier` and pass the data files (see `--help`). The data files can be obtained from releases. Alternatively, `hsmusic` can be obtained from https://notabug.org/hsmusic/hsmusic and `bandcamp.json` can be obtained from `dump_bandcamp`.

## Usage (CLI)
Run `cli`.

```
hsmusicifier 0.1.0
A tool to add track art to Homestuck music.

USAGE:
    cli [FLAGS] <in-dir> <out-dir> --bandcamp-json <bandcamp-json> --hsmusic <hsmusic>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbosity

OPTIONS:
    -b, --bandcamp-json <bandcamp-json>    Location of dumped bandcamp json
    -m, --hsmusic <hsmusic>                Location of hsmusic

ARGS:
    <in-dir>     Input directory
    <out-dir>    Output directory
```

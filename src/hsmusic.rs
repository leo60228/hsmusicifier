//! ported basically verbatim from original JS
use anyhow::{Context, Result};
use heck::KebabCase;
use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

#[derive(Debug, Copy, Clone)]
pub struct Contributor<'a> {
    pub who: &'a str,
    pub what: Option<&'a str>,
}

#[derive(Debug)]
pub struct Artist<'a> {
    pub name: &'a str,
    pub urls: Vec<&'a str>,
    pub alias: Option<&'a str>,
    pub note: Option<String>,
}

#[derive(Debug)]
pub struct Track<'a> {
    pub name: &'a str,
    pub commentary: Option<String>,
    pub lyrics: Option<String>,
    pub original_date: Option<&'a str>,
    pub cover_art_date: &'a str,
    pub references: Vec<&'a str>,
    pub artists: Vec<Contributor<'a>>,
    pub cover_artists: Option<Vec<Contributor<'a>>>,
    pub art_tags: Vec<&'a str>,
    pub contributors: Vec<Contributor<'a>>,
    pub directory: Cow<'a, str>,
    pub aka: Option<&'a str>,
    pub duration: usize,
    pub urls: Vec<&'a str>,
    pub group: &'a str,
    pub color: &'a str,
}

#[derive(Debug)]
pub struct Album<'a> {
    pub name: &'a str,
    pub artists: Option<Vec<Contributor<'a>>>,
    pub date: &'a str,
    pub track_art_date: &'a str,
    pub cover_art_date: &'a str,
    pub cover_artists: Option<Vec<Contributor<'a>>>,
    pub has_track_art: bool,
    pub track_cover_artists: Option<Vec<Contributor<'a>>>,
    pub art_tags: Vec<&'a str>,
    pub commentary: Option<String>,
    pub urls: Vec<&'a str>,
    pub groups: Vec<&'a str>,
    pub directory: Cow<'a, str>,
    pub is_major_release: bool,
    pub color: &'a str,
    pub uses_groups: bool,
    pub tracks: Vec<Track<'a>>,
}

fn get_basic_field<'a, 'b>(s: &'a str, name: &'b str) -> Option<&'a str> {
    if let Some(line) = s
        .lines()
        .find(|line| line.starts_with(&format!("{}:", name)))
    {
        Some(line[name.len() + 1..].trim())
    } else {
        None
    }
}

fn get_list_field<'a, 'b>(s: &'a str, name: &'b str) -> Option<Vec<&'a str>> {
    let mut start_index = if let Some(i) = s
        .lines()
        .position(|line| line.starts_with(&format!("{}:", name)))
    {
        i
    } else {
        return None;
    };

    let end_index = if let Some(i) = s
        .lines()
        .skip(start_index + 1)
        .position(|line| !line.starts_with("- "))
    {
        i + start_index
    } else {
        s.lines().count()
    };

    start_index += 1;

    if start_index == end_index {
        if let Some(value) = get_basic_field(s, name) {
            Some(value.split(',').map(|x| x.trim()).collect())
        } else {
            None
        }
    } else {
        Some(
            s.lines()
                .take(end_index)
                .skip(start_index)
                .map(|x| &x[2..])
                .collect(),
        )
    }
}

fn get_contribution_field<'a, 'b>(s: &'a str, name: &'b str) -> Option<Vec<Contributor<'a>>> {
    let contributors = get_list_field(s, name)?;

    const REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("^(.*?)( \\((.*)\\))?$").unwrap());
    let mapped: Vec<Contributor> = contributors
        .into_iter()
        .map(|contrib| {
            if let Some(captures) = REGEX.captures(contrib) {
                let who = captures.get(0)?.as_str();
                let what = captures.get(2).map(|x| x.as_str());

                Some(Contributor { who, what })
            } else {
                Some(Contributor {
                    who: contrib,
                    what: None,
                })
            }
        })
        .collect::<Option<_>>()?;

    if mapped.len() == 1 && mapped[0].who == "none" {
        None
    } else {
        Some(mapped)
    }
}

fn get_multiline_field<'a, 'b>(s: &'a str, name: &'b str) -> Option<String> {
    let mut start_index = if let Some(i) = s
        .lines()
        .position(|line| line.starts_with(&format!("{}:", name)))
    {
        i
    } else {
        return None;
    };

    let end_index = if let Some(i) = s
        .lines()
        .skip(start_index + 1)
        .position(|line| !line.starts_with("    "))
    {
        i + start_index
    } else {
        s.lines().count()
    };

    start_index += 1;

    if start_index == end_index {
        None
    } else {
        Some(
            s.lines()
                .take(end_index)
                .skip(start_index)
                .map(|x| &x[4..])
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

const SPLIT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("(?m)^-{8,}\n").unwrap());

pub fn parse_artist(string: &str) -> Result<Artist> {
    Ok(Artist {
        name: get_basic_field(string, "Artist").context("no Artist")?,
        urls: get_list_field(string, "URLs").unwrap_or_else(Vec::new),
        alias: get_basic_field(string, "Alias"),
        note: get_multiline_field(string, "Note"),
    })
}

pub fn parse_artists(string: &str) -> Result<Vec<Artist>> {
    SPLIT_REGEX.split(string).map(parse_artist).collect()
}

fn get_duration_in_seconds(duration: &str) -> usize {
    if let Some(parts) = duration
        .split(":")
        .map(|x| x.parse().ok())
        .collect::<Option<Vec<usize>>>()
    {
        match parts.len() {
            3 => parts[0] * 3600 + parts[1] * 60 + parts[2],
            2 => parts[0] * 60 + parts[1],
            _ => 0,
        }
    } else {
        0
    }
}

pub fn parse_track<'a>(
    section: &'a str,
    group: &'a str,
    group_color: &'a str,
    track_art_date: &'a str,
    artists: &Option<Vec<Contributor<'a>>>,
    color: &'a str,
) -> Result<Track<'a>> {
    let name = get_basic_field(section, "Track").context("missing Track")?;
    let original_date = get_basic_field(section, "Original Date");
    Ok(Track {
        name,
        commentary: get_multiline_field(section, "Commentary"),
        lyrics: get_multiline_field(section, "Lyrics"),
        original_date,
        cover_art_date: get_basic_field(section, "Cover Art Date")
            .or(original_date)
            .unwrap_or(track_art_date),
        references: get_list_field(section, "References").unwrap_or_default(),
        artists: get_contribution_field(section, "Artists")
            .or_else(|| get_contribution_field(section, "Artist"))
            .or_else(|| artists.clone())
            .context("missing Artists")?,
        cover_artists: match get_contribution_field(section, "Track Art") {
            Some(cover_artists) if cover_artists.len() > 0 && cover_artists[0].who == "none" => {
                None
            }
            Some(cover_artists) => Some(cover_artists),
            None => None,
        },
        art_tags: get_list_field(section, "Art Tags").unwrap_or_default(),
        contributors: get_contribution_field(section, "Contributors").unwrap_or_default(),
        directory: get_basic_field(section, "Directory")
            .map(From::from)
            .unwrap_or_else(|| name.to_kebab_case().into()),
        aka: get_basic_field(section, "AKA"),
        duration: get_duration_in_seconds(get_basic_field(section, "Duration").unwrap_or("0:00")),
        urls: get_list_field(section, "URLs").unwrap_or_default(),
        group,
        color: if !group.is_empty() {
            group_color
        } else {
            color
        },
    })
}

pub fn parse_album(string: &str) -> Result<Album> {
    let mut tracks = vec![];

    let regex = &*SPLIT_REGEX;
    let mut split = regex.split(string);
    let album_section = split.next().context("no album section")?;

    let name = get_basic_field(album_section, "Album").context("no Album")?;
    let artists = get_contribution_field(album_section, "Artists")
        .or_else(|| get_contribution_field(album_section, "Artist"));
    let date = get_basic_field(album_section, "Date").context("no Date")?;
    let color = get_basic_field(album_section, "FG").unwrap_or("#0088ff");
    let track_art_date = get_basic_field(album_section, "Track Art Date").unwrap_or(date);

    let mut uses_groups = false;
    let mut group = "";
    let mut group_color = color;

    for section in split {
        if section.trim().len() == 0 {
            continue;
        }

        if let Some(group_name) = get_basic_field(section, "Group") {
            group = group_name;
            group_color = get_basic_field(section, "FG").unwrap_or(color);
            uses_groups = true;
        } else {
            tracks.push(parse_track(
                section,
                group,
                group_color,
                track_art_date,
                &artists,
                color,
            )?);
        }
    }

    Ok(Album {
        name,
        artists,
        date,
        track_art_date,
        cover_art_date: get_basic_field(album_section, "Cover Art Date").unwrap_or(date),
        cover_artists: get_contribution_field(album_section, "Cover Art"),
        has_track_art: get_basic_field(album_section, "Has Track Art") != Some("no"),
        track_cover_artists: get_contribution_field(album_section, "Track Art"),
        art_tags: get_list_field(album_section, "Art Tags").unwrap_or_default(),
        commentary: get_multiline_field(album_section, "Commentary"),
        urls: get_list_field(album_section, "URLs").unwrap_or_default(),
        groups: get_list_field(album_section, "Groups").unwrap_or_default(),
        directory: get_basic_field(album_section, "Directory")
            .map(From::from)
            .unwrap_or_else(|| name.to_kebab_case().into()),
        is_major_release: get_basic_field(album_section, "Major Release") == Some("yes"),
        color,
        uses_groups,
        tracks,
    })
}
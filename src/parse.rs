//! ported basically verbatim from original JS
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct Artist<'a> {
    name: &'a str,
    urls: Vec<&'a str>,
    alias: Option<&'a str>,
    note: Option<String>,
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

fn parse_artist(string: &str) -> Option<Artist> {
    Some(Artist {
        name: get_basic_field(string, "Artist")?,
        urls: get_list_field(string, "URLs").unwrap_or_else(Vec::new),
        alias: get_basic_field(string, "Alias"),
        note: get_multiline_field(string, "Note"),
    })
}

pub fn parse_artists(string: &str) -> Option<Vec<Artist>> {
    let mut res = vec![];

    const REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("(?m)^-{8,}\n").unwrap());
    for string in REGEX.split(string) {
        res.push(parse_artist(string)?);
    }

    Some(res)
}

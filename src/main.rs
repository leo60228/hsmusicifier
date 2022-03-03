use anyhow::{anyhow, ensure, Context, Result};
use clap::Parser;
use hsmusicifier::{ArtType, ArtTypes, Edits};
use iui::{controls::*, prelude::*};
use nfd::Response;
use std::cell::RefCell;
use std::env::current_exe;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicIsize, AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::thread;

#[derive(Parser)]
#[clap(
    name = "hsmusicifier",
    about = "A tool to add track art to Homestuck music."
)]
struct Opt {
    /// Location of dumped bandcamp json
    #[clap(short, long = "bandcamp-json", parse(from_os_str))]
    pub bandcamp_json: Option<PathBuf>,

    /// Location of hsmusic-data
    #[clap(short = 'd', long, parse(from_os_str))]
    pub hsmusic_data: Option<PathBuf>,

    /// Location of hsmusic-media
    #[clap(short = 'm', long, parse(from_os_str))]
    pub hsmusic_media: Option<PathBuf>,
}

fn find_file(specified: Option<PathBuf>, name: &str) -> Result<PathBuf> {
    Ok(match specified {
        Some(x) => x,
        None => {
            let exe = current_exe()?;
            let dir = exe.parent().context("Bad $0!")?;

            let win_path = dir.join(name);

            if win_path.exists() {
                win_path
            } else {
                dir.parent().context("Bad $0!")?.join("share").join(name)
            }
        }
    })
}

fn run(ui: UI, mut win: Window) -> Result<()> {
    let Opt {
        bandcamp_json,
        hsmusic_data,
        hsmusic_media,
    } = Opt::parse();

    let bandcamp_json = find_file(bandcamp_json, "bandcamp.json")?;
    let hsmusic_data = find_file(hsmusic_data, "hsmusic-data")?;
    let hsmusic_media = find_file(hsmusic_media, "hsmusic-media")?;

    ensure!(bandcamp_json.is_file(), "Missing bandcamp.json!");
    ensure!(hsmusic_data.is_dir(), "Missing hsmusic-data!");
    ensure!(hsmusic_media.is_dir(), "Missing hsmusic-media!");

    let (tx, rx) = mpsc::channel();
    let progress = Arc::new(AtomicIsize::new(-1));
    let progress_total = Arc::new(AtomicUsize::new(0));
    let thread = Rc::new(RefCell::new(None));

    let mut select = VerticalBox::new(&ui);
    let mut add = VerticalBox::new(&ui);
    let finish = VerticalBox::new(&ui);

    // MUSIC PAGE

    let mut next_button = Button::new(&ui, "Next");

    let mut input_entry = Entry::new(&ui);
    let mut input_button = Button::new(&ui, "...");

    let mut input_chooser = HorizontalBox::new(&ui);

    input_chooser.set_padded(&ui, true);

    input_button.on_clicked(&ui, {
        let ui = ui.clone();
        let mut next_button = next_button.clone();
        let mut input_entry = input_entry.clone();
        move |_| {
            if let Ok(Response::Okay(file)) = nfd::open_pick_folder(None) {
                input_entry.set_value(&ui, &file);

                if Path::new(&file).is_dir() {
                    next_button.enable(&ui);
                } else {
                    next_button.disable(&ui);
                }
            }
        }
    });

    input_entry.on_changed(&ui, {
        let ui = ui.clone();
        let mut next_button = next_button.clone();
        move |path| {
            if Path::new(&path).is_dir() {
                next_button.enable(&ui);
            } else {
                next_button.disable(&ui);
            }
        }
    });

    input_chooser.append(&ui, input_entry.clone(), LayoutStrategy::Stretchy);
    input_chooser.append(&ui, input_button, LayoutStrategy::Compact);

    let output_entry = Entry::new(&ui);
    let mut output_button = Button::new(&ui, "...");

    let mut output_chooser = HorizontalBox::new(&ui);

    output_chooser.set_padded(&ui, true);

    output_button.on_clicked(&ui, {
        let ui = ui.clone();
        let mut output_entry = output_entry.clone();
        move |_| {
            if let Ok(Response::Okay(file)) = nfd::open_pick_folder(None) {
                output_entry.set_value(&ui, &file);
            }
        }
    });

    output_chooser.append(&ui, output_entry.clone(), LayoutStrategy::Stretchy);
    output_chooser.append(&ui, output_button, LayoutStrategy::Compact);

    let mut add_artists = Checkbox::new(&ui, "Add artists");
    add_artists.set_checked(&ui, true);

    let mut add_album = Checkbox::new(&ui, "Add album");
    add_album.set_checked(&ui, false);

    let mut add_art = Checkbox::new(&ui, "Add art");
    add_art.set_checked(&ui, true);

    let mut first_art = Combobox::new(&ui);
    first_art.append(&ui, "Album Art");
    first_art.append(&ui, "Track Art");
    first_art.set_selected(&ui, 0);

    let mut rest_art = Combobox::new(&ui);
    rest_art.append(&ui, "Album Art");
    rest_art.append(&ui, "Track Art");
    rest_art.set_selected(&ui, 1);

    let mut first_art_chooser = LayoutGrid::new(&ui);
    first_art_chooser.append(
        &ui,
        Label::new(&ui, "First song:"),
        0,
        1,
        1,
        1,
        GridExpand::Neither,
        GridAlignment::Start,
        GridAlignment::Center,
    );
    first_art_chooser.append(
        &ui,
        first_art.clone(),
        1,
        1,
        1,
        1,
        GridExpand::Both,
        GridAlignment::End,
        GridAlignment::Fill,
    );
    first_art_chooser.set_padded(&ui, true);

    let mut rest_art_chooser = LayoutGrid::new(&ui);
    rest_art_chooser.append(
        &ui,
        Label::new(&ui, "Other songs:"),
        0,
        1,
        1,
        1,
        GridExpand::Neither,
        GridAlignment::Start,
        GridAlignment::Center,
    );
    rest_art_chooser.append(
        &ui,
        rest_art.clone(),
        1,
        1,
        1,
        1,
        GridExpand::Both,
        GridAlignment::End,
        GridAlignment::Fill,
    );
    rest_art_chooser.set_padded(&ui, true);

    add_art.on_toggled(&ui, {
        let ui = ui.clone();
        let mut first_art = first_art.clone();
        let mut rest_art = rest_art.clone();
        move |add_art| {
            if add_art {
                first_art.enable(&ui);
                rest_art.enable(&ui);
            } else {
                first_art.disable(&ui);
                rest_art.disable(&ui);
            }
        }
    });

    next_button.on_clicked(&ui, {
        let ui = ui.clone();
        let mut win = win.clone();
        let thread = thread.clone();
        let add = add.clone();
        let progress = progress.clone();
        let progress_total = progress_total.clone();
        let add_artists = add_artists.clone();
        let add_album = add_album.clone();
        let add_art = add_art.clone();
        let first_art = first_art;
        let rest_art = rest_art;
        move |_| {
            let input_path = PathBuf::from(&input_entry.value(&ui));
            let output_path = PathBuf::from(&output_entry.value(&ui));

            win.set_child(&ui, add.clone());

            let progress = progress.clone();
            let progress_total = progress_total.clone();

            let edits = Edits {
                add_artists: add_artists.checked(&ui),
                add_art: if add_art.checked(&ui) {
                    Some(ArtTypes {
                        first: if first_art.selected(&ui) == 0 {
                            ArtType::AlbumArt
                        } else {
                            ArtType::TrackArt
                        },
                        rest: if rest_art.selected(&ui) == 0 {
                            ArtType::AlbumArt
                        } else {
                            ArtType::TrackArt
                        },
                    })
                } else {
                    None
                },
                add_album: add_album.checked(&ui),
            };

            let hsmusic_data = hsmusic_data.clone();
            let hsmusic_media = hsmusic_media.clone();
            let bandcamp_json = bandcamp_json.clone();
            let tx = tx.clone();
            thread.replace(Some(thread::spawn(
                move || match std::panic::catch_unwind(|| {
                    hsmusicifier::add_art(
                        bandcamp_json,
                        hsmusic_data,
                        hsmusic_media,
                        edits,
                        true,
                        input_path,
                        output_path,
                        |total| {
                            progress_total.store(total, Ordering::SeqCst);
                            progress.fetch_add(1, Ordering::SeqCst);
                        },
                    )
                }) {
                    Ok(Ok(())) => tx.send(Ok(())).unwrap(),
                    Ok(Err(err)) => tx.send(Err(err)).unwrap(),
                    Err(panic) => {
                        let msg = match panic.downcast_ref::<&'static str>() {
                            Some(s) => *s,
                            None => match panic.downcast_ref::<String>() {
                                Some(s) => &s[..],
                                None => "Box<Any>",
                            },
                        };

                        tx.send(Err(anyhow!("panicked at '{}'", msg))).unwrap();

                        std::panic::resume_unwind(panic);
                    }
                },
            )));
        }
    });

    select.append(
        &ui,
        Label::new(&ui, "Select input location:"),
        LayoutStrategy::Compact,
    );
    select.append(&ui, input_chooser, LayoutStrategy::Compact);
    select.append(&ui, HorizontalSeparator::new(&ui), LayoutStrategy::Compact);
    select.append(
        &ui,
        Label::new(&ui, "Select output location:"),
        LayoutStrategy::Compact,
    );
    select.append(&ui, output_chooser, LayoutStrategy::Compact);
    select.append(&ui, HorizontalSeparator::new(&ui), LayoutStrategy::Compact);
    select.append(&ui, add_artists, LayoutStrategy::Compact);
    select.append(&ui, add_album, LayoutStrategy::Compact);
    select.append(&ui, add_art, LayoutStrategy::Compact);
    select.append(&ui, first_art_chooser, LayoutStrategy::Compact);
    select.append(&ui, rest_art_chooser, LayoutStrategy::Compact);
    select.append(&ui, Spacer::new(&ui), LayoutStrategy::Stretchy);
    select.append(&ui, next_button, LayoutStrategy::Compact);

    select.set_padded(&ui, true);

    // ADD PAGE
    let mut progress_bar = ProgressBar::indeterminate(&ui);

    add.append(&ui, Label::new(&ui, "Adding..."), LayoutStrategy::Compact);
    add.append(&ui, progress_bar.clone(), LayoutStrategy::Compact);
    add.set_padded(&ui, true);

    // FINISH PAGE
    {
        let mut finish = finish.clone();
        let mut exit = Button::new(&ui, "Exit");
        exit.on_clicked(&ui, {
            let ui = ui.clone();
            move |_| {
                ui.quit();
            }
        });

        let mut label_holder = HorizontalBox::new(&ui);
        label_holder.append(&ui, Spacer::new(&ui), LayoutStrategy::Stretchy);
        label_holder.append(
            &ui,
            Label::new(&ui, "Metadata has been added!"),
            LayoutStrategy::Compact,
        );
        label_holder.append(&ui, Spacer::new(&ui), LayoutStrategy::Stretchy);

        finish.append(&ui, Spacer::new(&ui), LayoutStrategy::Stretchy);
        finish.append(&ui, label_holder, LayoutStrategy::Compact);
        finish.append(&ui, Spacer::new(&ui), LayoutStrategy::Stretchy);
        finish.append(&ui, exit, LayoutStrategy::Compact);

        finish.set_padded(&ui, true);
    }

    // DISPLAY WINDOW
    {
        let mut win = win.clone();
        win.set_child(&ui, select);
        win.show(&ui);
    }

    // EVENT LOOP
    let mut eloop = ui.event_loop();
    eloop.on_tick(&ui, {
        let ui = ui.clone();
        move || {
            let progress = progress.load(Ordering::SeqCst);
            let progress_total = progress_total.load(Ordering::SeqCst);
            if progress >= 0 {
                progress_bar.set_value(&ui, (progress as usize * 100 / progress_total) as u32);
            }

            match rx.try_recv() {
                Ok(Ok(())) => {
                    thread.borrow_mut().take().unwrap().join().unwrap();
                    win.set_child(&ui, finish.clone());
                }
                Ok(Err(err)) => {
                    win.modal_err(&ui, "Error", &err.to_string());
                    panic!("{:?}", err);
                }
                Err(_) => {}
            }
        }
    });
    eloop.run_delay(&ui, 200);

    Ok(())
}

fn main() {
    let ui = UI::init().unwrap();

    let win = Window::new(&ui, "hsmusicifier", 200, 300, WindowType::NoMenubar);

    if let Err(err) = run(ui.clone(), win.clone()) {
        win.modal_err(&ui, "Error", &err.to_string());
        panic!("{:?}", err);
    }
}

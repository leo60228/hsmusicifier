use anyhow::{anyhow, ensure, Context, Result};
use iui::{controls::*, prelude::*};
use nfd::Response;
use std::cell::RefCell;
use std::env::current_exe;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicI8, Ordering},
    mpsc, Arc,
};
use std::thread::{self, JoinHandle};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "hsmusicifier",
    about = "A tool to add track art to Homestuck music."
)]
struct Opt {
    /// Location of dumped bandcamp json
    #[structopt(short, long = "bandcamp-json", parse(from_os_str))]
    pub bandcamp_json: Option<PathBuf>,

    /// Location of hsmusic
    #[structopt(short = "m", long, parse(from_os_str))]
    pub hsmusic: Option<PathBuf>,
}

fn spawn_thread(
    thread: &RefCell<Option<JoinHandle<()>>>,
    bandcamp_json: PathBuf,
    hsmusic: PathBuf,
    input: PathBuf,
    output: PathBuf,
    progress: Arc<AtomicI8>,
    tx: mpsc::Sender<Result<()>>,
) {
    thread.replace(Some(thread::spawn(
        move || match std::panic::catch_unwind(|| {
            hsmusicifier::add_art(
                bandcamp_json,
                hsmusic,
                true,
                input,
                output,
                |done, total| {
                    progress.store((done * 100 / total) as i8, Ordering::SeqCst);
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

fn run(ui: UI, mut win: Window) -> Result<()> {
    let Opt {
        bandcamp_json,
        hsmusic,
    } = Opt::from_args();

    let bandcamp_json = match bandcamp_json {
        Some(x) => x,
        None => current_exe()?
            .parent()
            .context("Bad $0!")?
            .join("bandcamp.json"),
    };
    let hsmusic = match hsmusic {
        Some(x) => x,
        None => current_exe()?.parent().context("Bad $0!")?.join("hsmusic"),
    };

    ensure!(bandcamp_json.is_file(), "Missing bandcamp.json!");
    ensure!(hsmusic.is_dir(), "Missing hsmusic!");

    let (tx, rx) = mpsc::channel();
    let progress = Arc::new(AtomicI8::new(-1));
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

    next_button.on_clicked(&ui, {
        let ui = ui.clone();
        let mut win = win.clone();
        let thread = thread.clone();
        let add = add.clone();
        let progress = progress.clone();
        move |_| {
            let input_path = PathBuf::from(&input_entry.value(&ui));
            let output_path = PathBuf::from(&output_entry.value(&ui));

            win.set_child(&ui, add.clone());

            let progress = progress.clone();

            spawn_thread(
                &thread,
                bandcamp_json.clone(),
                hsmusic.clone(),
                input_path,
                output_path,
                progress,
                tx.clone(),
            );
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
            Label::new(&ui, "Cover art has been added!"),
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
            if progress >= 0 {
                progress_bar.set_value(&ui, progress as u32);
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

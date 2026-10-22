//! The Launcher: an eframe window that patches the game and starts it, or `--check` for
//! unattended runs.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

use eframe::egui;
use twilights_launcher::manifest::Manifest;
use twilights_launcher::settings::{Channel, Mode, Settings};
use twilights_launcher::status::{check_line, progress_text, status_for, Event, Status, Tone};
use twilights_launcher::update::{self, Done};
use twilights_launcher::{data, launch, swap};

const AMBER: egui::Color32 = egui::Color32::from_rgb(230, 160, 40);
const RED: egui::Color32 = egui::Color32::from_rgb(220, 70, 70);

fn install_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

fn settings_path(dir: &Path) -> PathBuf {
    dir.join("launcher.toml")
}

fn installed_tag(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("game").join("manifest.json")).ok()?;
    Manifest::parse(&text).ok().map(|m| m.tag)
}

enum Msg {
    Event(Event),
    Done(Done),
}

struct App {
    dir: PathBuf,
    settings: Settings,
    missing: Vec<String>,
    asked_for_folder: bool,
    rx: Option<Receiver<Msg>>,
    status: Option<Status>,
    release_tag: Option<String>,
    notes: String,
    progress: Option<(u64, Option<u64>)>,
    game_installed: bool,
}

impl App {
    fn new() -> App {
        let dir = install_dir();
        swap::clean_old_launcher(&dir);
        let settings = Settings::load(&settings_path(&dir)).unwrap_or_default();
        let mut app = App {
            game_installed: update::game_installed(&dir),
            dir,
            settings,
            missing: Vec::new(),
            asked_for_folder: false,
            rx: None,
            status: None,
            release_tag: None,
            notes: String::new(),
            progress: None,
        };
        app.revalidate_data();
        app
    }

    /// Checks the saved data folder. Returns whether it is valid; fills `missing` when not.
    fn revalidate_data(&mut self) -> bool {
        match data::check(Path::new(&self.settings.data)) {
            Ok(_) => {
                self.missing.clear();
                true
            }
            Err(missing) => {
                self.missing = missing;
                false
            }
        }
    }

    fn pick_folder(&mut self) {
        if let Some(picked) = rfd::FileDialog::new().pick_folder() {
            match data::check(&picked) {
                Ok(resolved) => {
                    self.settings.data = resolved.to_string_lossy().into_owned();
                    self.missing.clear();
                    self.save();
                }
                Err(missing) => self.missing = missing,
            }
        }
    }

    fn save(&self) {
        let _ = self.settings.save(&settings_path(&self.dir));
    }

    fn start_worker(&mut self, ctx: &egui::Context) {
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.progress = None;
        let dir = self.dir.clone();
        let settings = self.settings.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let done = update::run(&dir, &settings, update::Mode::Window, |e| {
                let _ = tx.send(Msg::Event(e));
                ctx.request_repaint();
            });
            let _ = tx.send(Msg::Done(done));
            ctx.request_repaint();
        });
    }

    fn drain(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rx else { return };
        let msgs: Vec<Msg> = rx.try_iter().collect();
        for msg in msgs {
            match msg {
                Msg::Done(Done::Relaunched) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Msg::Done(Done::Finished) => {}
                Msg::Event(event) => self.apply(event),
            }
        }
    }

    fn apply(&mut self, event: Event) {
        self.game_installed = update::game_installed(&self.dir);
        match &event {
            Event::Release { tag, notes } => {
                self.release_tag = Some(tag.clone());
                self.notes = notes.clone();
            }
            Event::Downloading { done, total, .. } => self.progress = Some((*done, *total)),
            _ => self.progress = None,
        }
        if let Some(status) = status_for(&event, self.game_installed) {
            self.status = Some(status);
        }
    }

    fn play(&mut self, ctx: &egui::Context) {
        self.save();
        let data = PathBuf::from(&self.settings.data);
        if launch::command(&self.dir, &data, &self.settings)
            .spawn()
            .is_ok()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            self.status = Some(Status {
                text: "Couldn't start the game".into(),
                tone: Tone::Error,
                play_enabled: false,
                retry: false,
            });
        }
    }

    fn version_line(&self) -> String {
        let installed = installed_tag(&self.dir).unwrap_or_else(|| "none".into());
        let latest = self.release_tag.as_deref().unwrap_or("?");
        match self.settings.channel {
            Channel::Live => format!("Installed {installed} · Live {latest}"),
            Channel::Testing => format!("Installed {installed} · Testing {latest}"),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.asked_for_folder {
            self.asked_for_folder = true;
            if !self.missing.is_empty() {
                self.pick_folder();
            }
        }
        if self.rx.is_none() && self.missing.is_empty() {
            self.start_worker(ctx);
        }
        self.drain(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(self.version_line());
            ui.add_space(6.0);
            if let Some(status) = &self.status {
                let text = egui::RichText::new(&status.text);
                ui.label(match status.tone {
                    Tone::Normal => text,
                    Tone::Warning => text.color(AMBER),
                    Tone::Error => text.color(RED),
                });
                if status.retry && ui.button("Retry").clicked() {
                    self.status = None;
                    self.start_worker(ctx);
                }
            }
            if let Some((done, total)) = self.progress {
                let fraction = total.map(|t| done as f32 / t.max(1) as f32).unwrap_or(0.0);
                ui.add(
                    egui::ProgressBar::new(fraction)
                        .text(progress_text(done, total))
                        .animate(total.is_none()),
                );
            }
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .auto_shrink([false, false])
                .show(ui, |ui| ui.label(&self.notes));
            ui.add_space(6.0);

            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("Data folder:");
                ui.label(if self.settings.data.is_empty() {
                    "(not set)"
                } else {
                    &self.settings.data
                });
                if ui.button("Change").clicked() {
                    changed = true;
                }
            });
            if !self.missing.is_empty() {
                ui.colored_label(RED, format!("Missing: {}", self.missing.join(", ")));
            }
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.settings.mode, Mode::Solo, "Solo");
                ui.radio_value(&mut self.settings.mode, Mode::Join, "Join");
            });
            if self.settings.mode == Mode::Join {
                ui.horizontal(|ui| {
                    ui.label("Server");
                    ui.text_edit_singleline(&mut self.settings.server);
                    ui.label("Password");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.join_password).password(true),
                    );
                });
            }
            ui.add_space(8.0);
            let enabled =
                self.missing.is_empty() && self.status.as_ref().is_some_and(|s| s.play_enabled);
            if ui
                .add_enabled(
                    enabled,
                    egui::Button::new("Play").min_size(egui::vec2(120.0, 32.0)),
                )
                .clicked()
            {
                self.play(ctx);
            }
            if changed {
                self.pick_folder();
                if self.missing.is_empty() {
                    self.start_worker(ctx);
                }
            }
        });
    }
}

/// A release build has no console, so `--check` attaches to the parent's to print.
#[cfg(all(windows, not(debug_assertions)))]
fn attach_console() {
    use std::os::windows::io::AsRawHandle;
    extern "system" {
        fn AttachConsole(pid: u32) -> i32;
        fn GetStdHandle(which: u32) -> isize;
        fn SetStdHandle(which: u32, handle: isize) -> i32;
    }
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    // SAFETY: plain Win32 calls with valid arguments; the CONOUT$ file is leaked on purpose so
    // the handle stays open for the process lifetime.
    unsafe {
        let current = GetStdHandle(STD_OUTPUT_HANDLE);
        if current != 0 && current != -1 {
            return;
        }
        if AttachConsole(u32::MAX) == 0 {
            return;
        }
        if let Ok(out) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
            SetStdHandle(STD_OUTPUT_HANDLE, out.as_raw_handle() as isize);
            std::mem::forget(out);
        }
    }
}

#[cfg(not(all(windows, not(debug_assertions))))]
fn attach_console() {}

fn check_mode() -> ! {
    attach_console();
    let dir = install_dir();
    swap::clean_old_launcher(&dir);
    let settings = match Settings::load(&settings_path(&dir)) {
        Ok(s) => s,
        Err(e) => {
            println!("error {e}");
            std::process::exit(1);
        }
    };
    let mut code = 1;
    update::run(&dir, &settings, update::Mode::Check, |event| {
        if let Event::LauncherUpdate(tag) = &event {
            println!("launcher-update {tag}");
        }
        if let Some((line, c)) = check_line(&event, update::game_installed(&dir)) {
            println!("{line}");
            code = c;
        }
    });
    std::process::exit(code);
}

fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--check") {
        check_mode();
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 420.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Twilight's Eve",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

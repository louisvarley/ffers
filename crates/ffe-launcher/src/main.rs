//! The Classic launcher: verify original game files, edit shared settings
//! and start the game. The look
//! follows the approved launcher mockup (`assets/`): a 1536x1024 design
//! canvas, scaled to the window, over the cleaned mockup artwork.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod checks;
mod config;
mod draw;
mod update;

use checks::{Kind, Level, Report};
use config::{Launcher, Videos};
use draw::{Icon, CREAM, CYAN, DIM, OLIVE, PANEL, VERIFIED};
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, Rect, RichText, Sense, Stroke, TextureHandle,
    UiBuilder, Vec2,
};
use ffe_rs::display::{Filter, FitMode, PixelAspect};
use ffe_rs::prefs;
use ffe_rs::settings::{self, Config};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver};

/// The design canvas (the mockup's size).
const CANVAS: Vec2 = vec2(1536.0, 1024.0);
const WARNING: Color32 = Color32::from_rgb(240, 190, 70);
const FAILED: Color32 = Color32::from_rgb(255, 96, 80);
/// Corner radii of the sidebar panel and of its items.
const SIDEBAR_RADIUS: f32 = 18.0;
const ITEM_RADIUS: f32 = 12.0;

/// The project's home on GitHub.
const GITHUB_URL: &str = "https://github.com/louisvarley/ffe-rs-classic";

/// The game's console output from the last launch.
const GAME_LOG: &str = "last-run.log";
/// The launcher's own errors (a Windows GUI program has no console).
const LAUNCHER_LOG: &str = "launcher.log";

fn main() {
    // A GUI program on Windows has no console: a panic would vanish without
    // a word. Log it beside the saves and show it.
    std::panic::set_hook(Box::new(|info| {
        let message = format!("The launcher stopped unexpectedly:\n\n{info}");
        report_fatal(&message);
    }));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Frontier: First Encounters (Elite 3) Launcher")
            .with_inner_size([1152.0, 768.0])
            .with_min_inner_size([768.0, 512.0])
            .with_icon(draw::window_icon()),
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "ffe-rs Launcher",
        options,
        Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)))),
    ) {
        report_fatal(&format!(
            "The launcher could not open its window.\n\n{error}"
        ));
        std::process::exit(1);
    }
}

/// Write `message` to `saves/launcher.log` and show it (a message box on
/// Windows and macOS, standard error elsewhere).
fn report_fatal(message: &str) {
    let saves = config::find_root().join("saves");
    let _ = std::fs::create_dir_all(&saves);
    let _ = std::fs::write(saves.join(LAUNCHER_LOG), message);
    eprintln!("{message}");
    #[cfg(any(windows, target_os = "macos"))]
    {
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Frontier: First Encounters Launcher")
            .set_description(message)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Play,
    Settings,
    About,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Display,
    Game,
    Enhancements,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Job {
    Game,
}

/// Where the check for a newer GitHub release has got to.
enum UpdateState {
    /// Not looked yet (the check is off, or has not started).
    Idle,
    Checking(Receiver<Result<update::Release, String>>),
    UpToDate,
    Available(update::Release),
    Installing(Receiver<Result<usize, String>>, update::Release),
    Failed(String),
}

struct App {
    root: PathBuf,
    saves: PathBuf,
    launcher: Launcher,
    /// Where the game videos are, and how many (AVI files).
    videos: Videos,
    config: Config,
    page: Page,
    tab: Tab,
    game_text: String,
    game_report: Option<Report>,
    jobs: Vec<(Job, Receiver<Report>)>,
    child: Option<Child>,
    notice: Option<(Level, String)>,
    background: TextureHandle,
    /// The folder the launcher's program files are in (and updates are written to).
    install_dir: PathBuf,
    update: UpdateState,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        draw::fonts(ctx);
        draw::style(ctx);
        let background = draw::background(ctx);
        let root = config::find_root();
        let saves = root.join("saves");
        let launcher = Launcher::load(&saves, &root);
        let config = settings::load_config(&saves).unwrap_or_default();
        let videos = config::resolve_videos(&launcher.game, launcher.videos.as_deref());
        let mut app = Self {
            game_text: launcher.game.display().to_string(),
            root,
            saves,
            launcher,
            videos,
            config,
            page: Page::Play,
            tab: Tab::Display,
            game_report: None,
            jobs: Vec::new(),
            child: None,
            notice: None,
            background,
            install_dir: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from(".")),
            update: UpdateState::Idle,
        };
        update::remove_leftovers(&app.install_dir);
        app.start(Job::Game);
        if app.launcher.check_updates {
            app.check_for_update();
        }
        app
    }

    /// Ask GitHub, in the background, whether a newer release exists.
    fn check_for_update(&mut self) {
        if matches!(
            self.update,
            UpdateState::Checking(_) | UpdateState::Installing(..)
        ) {
            return;
        }
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = send.send(update::fetch_latest());
        });
        self.update = UpdateState::Checking(receive);
    }

    /// Download and install `release`, then (in [`Self::poll`]) restart the launcher.
    fn install_update(&mut self, release: update::Release) {
        if self.child.is_some() {
            self.notice = Some((Level::Warning, "Close the game before updating.".to_owned()));
            return;
        }
        let (send, receive) = mpsc::channel();
        let install = self.install_dir.clone();
        let job = release.clone();
        std::thread::spawn(move || {
            let _ = send.send(update::download_and_install(&job, &install));
        });
        self.update = UpdateState::Installing(receive, release);
    }

    fn poll_update(&mut self) {
        let state = std::mem::replace(&mut self.update, UpdateState::Idle);
        self.update = match state {
            UpdateState::Checking(receive) => match receive.try_recv() {
                Ok(Ok(release)) if update::is_newer(update::current_version(), release.version) => {
                    UpdateState::Available(release)
                }
                Ok(Ok(_)) => UpdateState::UpToDate,
                Ok(Err(error)) => UpdateState::Failed(error),
                Err(mpsc::TryRecvError::Empty) => UpdateState::Checking(receive),
                Err(mpsc::TryRecvError::Disconnected) => {
                    UpdateState::Failed("The update check stopped.".to_owned())
                }
            },
            UpdateState::Installing(receive, release) => match receive.try_recv() {
                Ok(Ok(_)) => {
                    // The new launcher is in place: start it and leave.
                    if let Some(exe) = std::env::current_exe()
                        .ok()
                        .map(|exe| self.install_dir.join(exe.file_name().unwrap_or_default()))
                    {
                        if Command::new(exe).spawn().is_ok() {
                            std::process::exit(0);
                        }
                    }
                    self.notice = Some((
                        Level::Good,
                        format!("Updated to {}. Restart the launcher.", release.tag),
                    ));
                    UpdateState::UpToDate
                }
                Ok(Err(error)) => {
                    self.notice = Some((Level::Bad, error.clone()));
                    UpdateState::Available(release)
                }
                Err(mpsc::TryRecvError::Empty) => UpdateState::Installing(receive, release),
                Err(mpsc::TryRecvError::Disconnected) => {
                    UpdateState::Failed("The update stopped unexpectedly.".to_owned())
                }
            },
            other => other,
        };
    }

    /// What the update button does for `release`: install it when this is a packaged Windows
    /// release with a package to download, else open its page.
    fn update_action(&mut self, release: update::Release) {
        if update::can_install(&self.install_dir) && release.asset_url.is_some() {
            self.install_update(release);
        } else {
            open_url(&release.page);
        }
    }

    fn busy(&self, job: Job) -> bool {
        self.jobs.iter().any(|(j, _)| *j == job)
    }

    fn start(&mut self, job: Job) {
        if self.busy(job) {
            return;
        }
        let (send, receive) = mpsc::channel();
        let game = self.launcher.game.clone();
        let music_bin = self.root.join("music.bin");
        std::thread::spawn(move || {
            let report = match job {
                Job::Game => checks::verify_game_and_prepare_music(&game, &music_bin),
            };
            let _ = send.send(report);
        });
        self.jobs.push((job, receive));
    }

    fn game_ok(&self) -> bool {
        self.game_report
            .as_ref()
            .is_some_and(|report| report.level != Level::Bad)
    }

    fn poll(&mut self) {
        self.poll_update();
        let mut finished = Vec::new();
        self.jobs.retain(|(job, receive)| match receive.try_recv() {
            Ok(report) => {
                finished.push((*job, report));
                false
            }
            Err(mpsc::TryRecvError::Empty) => true,
            Err(mpsc::TryRecvError::Disconnected) => false,
        });
        for (job, report) in finished {
            match job {
                Job::Game => self.game_report = Some(report),
            }
        }
        if let Some(child) = &mut self.child {
            if let Ok(Some(status)) = child.try_wait() {
                self.notice = Some(if status.success() {
                    (Level::Good, "The game has closed.".to_owned())
                } else {
                    // The game prints its reason last (see About > game log).
                    let reason = std::fs::read_to_string(self.saves.join(GAME_LOG))
                        .ok()
                        .and_then(|log| {
                            log.lines()
                                .map(str::trim)
                                .rfind(|line| !line.is_empty() && !line.starts_with("Press Enter"))
                                .map(str::to_owned)
                        });
                    match reason {
                        Some(reason) => (Level::Bad, format!("The game stopped: {reason}")),
                        None => (Level::Bad, format!("The game stopped with {status}.")),
                    }
                });
                self.child = None;
                // The game saves its own options; pick up what it changed.
                if let Some(config) = settings::load_config(&self.saves) {
                    self.config = config;
                }
            }
        }
    }

    fn verify_game_folder(&mut self, path: PathBuf) {
        let next = config::detect_game(&path).unwrap_or(path);
        if let Err(error) = config::invalidate_assets(&self.root, &self.launcher.game, &next) {
            self.notice = Some((Level::Bad, error));
            return;
        }
        self.jobs.clear(); // Discard results for the previously selected folder.
        self.launcher.game = next;
        self.game_text = self.launcher.game.display().to_string();
        self.refresh_videos();
        self.save_launcher();
        self.game_report = None;
        self.start(Job::Game);
    }

    /// Look for the videos again: the player's folder, else the game's `DATA`.
    fn refresh_videos(&mut self) {
        self.videos = config::resolve_videos(&self.launcher.game, self.launcher.videos.as_deref());
    }

    /// Use `picked` (or the `DATA` folder inside it) for the videos, or say why not.
    fn choose_videos(&mut self, picked: PathBuf) {
        let folder = ffe_rs::fmv::video_folder(&picked);
        if ffe_rs::fmv::count_avi(&folder) == 0 {
            self.notice = Some((
                Level::Warning,
                format!("No AVI files in {}.", folder.display()),
            ));
            return;
        }
        self.launcher.videos = Some(folder);
        self.refresh_videos();
        self.save_launcher();
        self.notice = None;
    }

    /// Go back to the game folder's own `DATA` folder for the videos.
    fn auto_videos(&mut self) {
        self.launcher.videos = None;
        self.refresh_videos();
        self.save_launcher();
    }

    fn save_launcher(&mut self) {
        if let Err(error) = self.launcher.save(&self.saves) {
            self.notice = Some((Level::Bad, format!("Could not save launcher.cfg: {error}")));
        }
    }

    fn save_config(&mut self) {
        let result = std::fs::create_dir_all(&self.saves)
            .map_err(|e| e.to_string())
            .and_then(|()| settings::save_config(&self.saves, &self.config));
        if let Err(error) = result {
            self.notice = Some((Level::Bad, format!("Could not save config.cfg: {error}")));
        }
    }

    fn launch(&mut self) {
        if !self.game_ok() || self.busy(Job::Game) || self.child.is_some() {
            return;
        }
        self.save_config();
        let Some(game) = config::game_executable() else {
            self.notice = Some((
                Level::Bad,
                "ffe-rs was not found beside the launcher.".to_owned(),
            ));
            return;
        };
        let mut command = Command::new(game);
        command.arg("--root").arg(&self.root);
        command.arg("--game").arg(&self.launcher.game);
        command.arg("--start").arg(self.launcher.start.to_string());
        if let Some(videos) = self.videos.folder.as_ref().filter(|_| self.videos.found()) {
            command.arg("--videos").arg(videos);
        }
        if self.launcher.skip_intro {
            command.arg("--skip-intro");
        }
        if self.launcher.record {
            let dir = self.root.join("recordings");
            let _ = std::fs::create_dir_all(&dir);
            let seconds = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            command
                .arg("--record")
                .arg(dir.join(format!("{}.ffe-input", config::stamp(seconds))));
        }
        command.current_dir(&self.root);
        // No console window: the game's messages go to saves/last-run.log,
        // and with no terminal on stdin it exits at once on an error instead
        // of waiting for Enter.
        let _ = std::fs::create_dir_all(&self.saves);
        command.stdin(std::process::Stdio::null());
        if let Ok(log) = std::fs::File::create(self.saves.join(GAME_LOG)) {
            if let Ok(copy) = log.try_clone() {
                command.stdout(copy);
            }
            command.stderr(log);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        match command.spawn() {
            Ok(child) => {
                self.child = Some(child);
                self.notice = Some((Level::Good, "The game is running.".to_owned()));
            }
            Err(error) => {
                self.notice = Some((Level::Bad, format!("Could not start the game: {error}")));
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        if !self.jobs.is_empty()
            || self.child.is_some()
            || matches!(
                self.update,
                UpdateState::Checking(_) | UpdateState::Installing(..)
            )
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        // Scale the design canvas to the window (letterboxed).
        let native = ctx.input(|i| i.screen_rect().size()) * ctx.zoom_factor();
        let zoom = (native.x / CANVAS.x).min(native.y / CANVAS.y).max(0.3);
        if (zoom - ctx.zoom_factor()).abs() > 0.001 {
            ctx.set_zoom_factor(zoom);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let screen = ui.max_rect();
                let canvas = Rect::from_center_size(screen.center(), CANVAS);
                let at = |x: f32, y: f32| canvas.min + vec2(x, y);
                let rect =
                    |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(at(x0, y0), at(x1, y1));
                ui.painter().image(
                    self.background.id(),
                    canvas,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                self.sidebar(ui, &rect);
                match self.page {
                    Page::Play => self.play(ui, &rect),
                    Page::Settings => self.settings(ui, rect(309.0, 372.0, 1515.0, 932.0)),
                    Page::About => self.about(ui, rect(309.0, 372.0, 1515.0, 932.0)),
                }
                self.footer(ui, &at);
            });
    }
}

// --- Frame: sidebar and footer -------------------------------------------

impl App {
    fn sidebar(&mut self, ui: &mut egui::Ui, rect: &dyn Fn(f32, f32, f32, f32) -> Rect) {
        let painter = ui.painter().clone();
        // The sidebar's own translucent olive panel, with rounded corners.
        let frame = rect(22.0, 372.0, 287.0, 945.0);
        draw::chamfer_gradient(
            &painter,
            frame,
            SIDEBAR_RADIUS,
            &[
                (0.0, Color32::from_rgba_unmultiplied(26, 34, 8, 210)),
                (0.6, Color32::from_rgba_unmultiplied(10, 16, 8, 200)),
                (1.0, Color32::from_rgba_unmultiplied(30, 40, 8, 120)),
            ],
        );
        painter.rect_stroke(
            frame,
            SIDEBAR_RADIUS,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(220, 225, 200, 90)),
            egui::StrokeKind::Inside,
        );
        let entries = [
            (Some(Page::Play), "Play", Icon::Play, 388.0),
            (Some(Page::Settings), "Settings", Icon::Settings, 448.0),
            (Some(Page::About), "About", Icon::Info, 508.0),
            // A link, not a page: pinned to the bottom of the panel.
            (None, "GitHub", Icon::Code, 877.0),
        ];
        // A thin divider above the link.
        painter.line_segment(
            [
                rect(46.0, 862.0, 0.0, 0.0).min,
                rect(263.0, 862.0, 0.0, 0.0).min,
            ],
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(220, 225, 200, 60)),
        );
        for (index, (page, label, icon, top)) in entries.into_iter().enumerate() {
            let item = rect(34.0, top, 275.0, top + 54.0);
            let response = ui.interact(item, ui.id().with(("nav", index)), Sense::click());
            let selected = page.is_some_and(|page| self.page == page);
            if selected {
                draw::chamfer_gradient(
                    &painter,
                    item,
                    ITEM_RADIUS,
                    &[
                        (0.0, Color32::from_rgb(150, 164, 20)),
                        (0.5, Color32::from_rgb(98, 112, 6)),
                        (1.0, Color32::from_rgb(44, 54, 2)),
                    ],
                );
                painter.rect_stroke(
                    item,
                    ITEM_RADIUS,
                    Stroke::new(1.5_f32, CREAM),
                    egui::StrokeKind::Inside,
                );
            } else if response.hovered() {
                painter.rect_filled(
                    item,
                    ITEM_RADIUS,
                    Color32::from_rgba_unmultiplied(120, 130, 30, 55),
                );
            }
            let centre = pos2(item.left() + 36.0, item.center().y);
            draw::icon(
                &painter,
                icon,
                Rect::from_center_size(centre, Vec2::splat(34.0)),
                CREAM,
            );
            painter.text(
                pos2(item.left() + 76.0, item.center().y),
                Align2::LEFT_CENTER,
                label,
                draw::font(26.0, draw::BOLD),
                CREAM,
            );
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            let response = if page.is_none() {
                response.on_hover_text(GITHUB_URL)
            } else {
                response
            };
            if response.clicked() {
                match page {
                    Some(page) => self.page = page,
                    None => open_url(GITHUB_URL),
                }
            }
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui, at: &dyn Fn(f32, f32) -> egui::Pos2) {
        let painter = ui.painter().clone();
        // A newer release: one click installs it (or opens its page where it cannot).
        let button = Rect::from_center_size(at(830.0, 978.0), vec2(380.0, 30.0));
        match &self.update {
            UpdateState::Available(release) => {
                let label = if update::can_install(&self.install_dir) && release.asset_url.is_some()
                {
                    format!("Update to {} - click to install", release.tag)
                } else {
                    format!("{} is available - click for the page", release.tag)
                };
                let release = release.clone();
                if ui
                    .put(
                        button,
                        egui::Button::new(RichText::new(label).color(WARNING)),
                    )
                    .clicked()
                {
                    self.update_action(release);
                }
            }
            UpdateState::Installing(_, release) => {
                painter.text(
                    button.center(),
                    Align2::CENTER_CENTER,
                    format!("Installing {}...", release.tag),
                    draw::font(19.0, draw::ITALIC),
                    WARNING,
                );
            }
            _ => {}
        }
        painter.text(
            at(1508.0, 978.0),
            Align2::RIGHT_CENTER,
            format!(
                "Frontier: First Encounters (Elite 3) Launcher v{}",
                env!("CARGO_PKG_VERSION")
            ),
            draw::font(19.0, draw::ITALIC),
            CREAM,
        );
        if let Some((level, text)) = &self.notice {
            painter.text(
                at(330.0, 978.0),
                Align2::LEFT_CENTER,
                text,
                draw::font(19.0, draw::ITALIC),
                level_colour(*level),
            );
        }
    }
}

fn level_colour(level: Level) -> Color32 {
    match level {
        Level::Good => VERIFIED,
        Level::Warning => WARNING,
        Level::Bad => FAILED,
    }
}

// --- Settings ------------------------------------------------------------

impl App {
    fn settings(&mut self, ui: &mut egui::Ui, panel: Rect) {
        draw::rounded_panel(
            ui.painter(),
            panel,
            16.0,
            Color32::from_rgba_unmultiplied(7, 18, 15, 236),
            Stroke::new(1.5_f32, Color32::from_rgb(215, 222, 210)),
        );
        let inner = panel.shrink2(vec2(30.0, 22.0));
        ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Settings")
                        .font(draw::font(30.0, draw::BOLD))
                        .color(CREAM),
                );
                ui.add_space(20.0);
                for (tab, label) in [
                    (Tab::Display, "Display"),
                    (Tab::Game, "Game options"),
                    (Tab::Enhancements, "Enhancements"),
                ] {
                    if ui.add(pill(label, self.tab == tab)).clicked() {
                        self.tab = tab;
                    }
                }
            });
            ui.add_space(4.0);
            let running = self.child.is_some();
            ui.label(
                RichText::new(if running {
                    "The game is running: it saves its own options when it closes."
                } else {
                    "Saved to saves/config.cfg, the file the in-game options pages use."
                })
                .italics()
                .color(if running { WARNING } else { DIM }),
            );
            ui.add_space(6.0);
            let before = self.config.clone();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_enabled_ui(!running, |ui| match self.tab {
                        Tab::Display => self.display_tab(ui),
                        Tab::Game => self.game_tab(ui),
                        Tab::Enhancements => self.enhancements_tab(ui),
                    });
                    {
                        ui.add_space(10.0);
                        if ui
                            .add_enabled(!running, egui::Button::new("Restore defaults"))
                            .on_hover_text("Key bindings (bind.*) are kept.")
                            .clicked()
                        {
                            let defaults = Config::default();
                            match self.tab {
                                Tab::Display => self.config.display = defaults.display,
                                Tab::Game => {
                                    self.config.modern = defaults.modern;
                                    self.config.others = defaults.others;
                                    self.config.maps = defaults.maps;
                                }
                                Tab::Enhancements => {
                                    self.config.enhancements = defaults.enhancements;
                                }
                            }
                        }
                    }
                });
            if self.config != before {
                self.save_config();
            }
        });
    }

    fn display_tab(&mut self, ui: &mut egui::Ui) {
        let display = &mut self.config.display;
        section(ui, "Window");
        option_row(
            ui,
            "Window size",
            "The starting window, in multiples of 320x240.",
            |ui| {
                let mut index = display.scale.clamp(1, 6) as usize - 1;
                if combo(
                    ui,
                    "scale",
                    &mut index,
                    &prefs::WINDOW_SCALE_LABELS.map(str::to_owned),
                ) {
                    display.scale = index as u32 + 1;
                }
            },
        );
        option_row(
            ui,
            "Full screen",
            "Start in a borderless full-screen window. Alt+Enter switches while playing.",
            |ui| {
                toggle(ui, &mut self.config.fullscreen);
            },
        );
        section(ui, "Video filter");
        filter_list(ui, &mut display.filter);
        section(ui, "Scaling");
        option_row(
            ui,
            "Fit",
            "Whole-pixel scaling with borders, or fill the window.",
            |ui| {
                choice(
                    ui,
                    &mut display.fit,
                    &[
                        (FitMode::PixelPerfect, prefs::FIT_LABELS[0]),
                        (FitMode::FillWindow, prefs::FIT_LABELS[1]),
                    ],
                );
            },
        );
        option_row(
            ui,
            "Pixel aspect",
            "The original's 4:3 monitor, or square pixels.",
            |ui| {
                choice(
                    ui,
                    &mut display.aspect,
                    &[
                        (PixelAspect::Dos43, prefs::ASPECT_LABELS[0]),
                        (PixelAspect::Square, prefs::ASPECT_LABELS[1]),
                    ],
                );
            },
        );
        section(ui, "CRT effect");
        option_row(
            ui,
            "Scanlines",
            "Dark lines between the picture's rows, like the original's monitor.",
            |ui| {
                toggle(ui, &mut display.scanlines);
            },
        );
        if display.scanlines {
            option_row(ui, "Scanline strength", "How dark the lines are.", |ui| {
                let mut percent = (display.scanline_intensity * 100.0).round() as u32;
                choice(
                    ui,
                    &mut percent,
                    &[
                        (20, "20%"),
                        (30, "30%"),
                        (50, "50%"),
                        (70, "70%"),
                        (90, "90%"),
                    ],
                );
                display.scanline_intensity = percent as f32 / 100.0;
            });
        }
    }

    fn game_tab(&mut self, ui: &mut egui::Ui) {
        self.videos_section(ui);
        let config = &mut self.config;
        section(ui, "Controls");
        option_row(
            ui,
            "Control keys",
            if config.modern {
                MODERN_CONTROLS
            } else {
                CLASSIC_CONTROLS
            },
            |ui| {
                choice(
                    ui,
                    &mut config.modern,
                    &[(false, "Classic"), (true, "Modern")],
                );
            },
        );
        section(ui, "Other options");
        toggle_grid(ui, "others", &mut config.others, &prefs::OTHER_LABELS);
        section(ui, "Galaxy map");
        toggle_grid(ui, "maps", &mut config.maps, &prefs::MAP_LABELS);
    }

    /// One toggle per optional enhancement, grouped under a header
    /// (`docs/notes/enhancements.md`). The settings page saves `config.cfg`
    /// whenever a toggle changes, so these keys go to the file the game reads.
    fn enhancements_tab(&mut self, ui: &mut egui::Ui) {
        section(ui, "Optional enhancements");
        ui.label(
            RichText::new(
                "Each improvement is off unless switched on. They are saved as \
                 enhancement.<name>=on|off in config.cfg and never in a commander save, \
                 so a toggle can be changed between games without affecting one.",
            )
            .color(DIM)
            .italics(),
        );
        for group in ENHANCEMENT_GROUPS {
            section(ui, group.heading);
            for item in group.items {
                let value = self
                    .config
                    .enhancements
                    .slot(item.slot)
                    .expect("one toggle per listed enhancement");
                option_row(ui, item.title, item.about, |ui| {
                    toggle(ui, value);
                });
            }
        }
    }

    /// Where the videos are, with a way to pick another folder and to go back
    /// to the game folder's own `DATA` folder.
    fn videos_section(&mut self, ui: &mut egui::Ui) {
        section(ui, "Game videos");
        let about = match (&self.videos.folder, self.videos.found()) {
            (Some(folder), true) => {
                format!("{} AVI files in {}", self.videos.count, folder.display())
            }
            (Some(folder), false) => format!("No AVI files in {}", folder.display()),
            (None, _) => "None found: the game folder has no DATA folder. Browse to the CD-ROM's DATA folder."
                .to_owned(),
        };
        let custom = self.videos.custom;
        let start = self
            .videos
            .folder
            .clone()
            .unwrap_or_else(|| self.launcher.game.clone());
        let mut picked = None;
        let mut reset = false;
        option_row(ui, "Video folder", &about, |ui| {
            if ui.button("Browse...").clicked() {
                picked = pick_folder("Choose the folder with the game's AVI videos", &start);
            }
            if custom && ui.button("Use the game's DATA folder").clicked() {
                reset = true;
            }
        });
        if let Some(path) = picked {
            self.choose_videos(path);
        }
        if reset {
            self.auto_videos();
        }
    }
}

/// What the control keys are, as the Settings row says it for each preset.
const CLASSIC_CONTROLS: &str = "The original's keys. Hold the right mouse button and drag to steer; \
Enter and right Shift raise and lower the speed; A and Z pitch, comma and full stop yaw, [ and ] roll.";
const MODERN_CONTROLS: &str = "W and S raise and lower the speed, the arrow keys pitch and yaw, \
Q and E roll. The mouse does not steer.";

/// One Enhancement toggle row: `slot` is its position in
/// `ffe_rs::settings::Enhancements::KEYS`, with the title and description the
/// launcher shows.
struct Enhancement {
    slot: usize,
    title: &'static str,
    about: &'static str,
}

/// A titled group of related enhancements, in display order; the headers follow
/// `docs/notes/enhancements.md`.
struct EnhancementGroup {
    heading: &'static str,
    items: &'static [Enhancement],
}

const ENHANCEMENT_GROUPS: [EnhancementGroup; 4] = [
    EnhancementGroup {
        heading: "Combat & missiles",
        items: &[
            Enhancement {
                slot: 0,
                title: "Turrets turn while time is paused",
                about: "Turrets keep tracking their target while time is paused.",
            },
            Enhancement {
                slot: 1,
                title: "Missiles ignore their launcher on launch",
                about: "A missile's collision ray ignores the ship that fired it, so it never \
                        hits its own launcher as it leaves.",
            },
            Enhancement {
                slot: 5,
                title: "Missiles use swept collision",
                about: "A missile's collision is tested along its whole path for the frame, so \
                        a fast missile cannot step past a thin target between two checks.",
            },
            Enhancement {
                slot: 7,
                title: "ECM can disable NN500 missiles",
                about: "The ECM can stop NN500-series missiles as well as the standard models.",
            },
        ],
    },
    EnhancementGroup {
        heading: "Autopilot",
        items: &[
            Enhancement {
                slot: 2,
                title: "Autopilot offsets its waypoint",
                about: "Before each leg the autopilot tests the path against nearby bodies and \
                        adds an offset waypoint when the direct path would hit one.",
            },
            Enhancement {
                slot: 3,
                title: "Autopilot braking uses retro thrust and gravity",
                about: "The braking point is calculated from the weaker retro thrust minus the \
                        local gravity, independently of the time step, so approaches do not \
                        crash.",
            },
            Enhancement {
                slot: 4,
                title: "Autopilot avoids every nearby body",
                about: "The autopilot tests collisions against every body in range, not just the \
                        current reference body.",
            },
        ],
    },
    EnhancementGroup {
        heading: "Mission fixes",
        items: &[Enhancement {
            slot: 6,
            title: "Mission fixes",
            about: "Mission corrections from the known-issues list: assassination kill credit, \
                    hitch-hiker completion, early medals, the hidden Alioth deadline, blocked \
                    docking bays, and mission dialogues under accelerated time.",
        }],
    },
    EnhancementGroup {
        heading: "Combat display",
        items: &[
            Enhancement {
                slot: 8,
                title: "NPC relative speed",
                about: "The selected NPC's hull and shield display also shows its speed relative \
                        to the player.",
            },
            Enhancement {
                slot: 9,
                title: "Combat Targeting Computer",
                about: "Adds new dashboard button for seeing local ships and auto targeting \
                        them.",
            },
            Enhancement {
                slot: 10,
                title: "Edge target indicator",
                about: "An arrow or bar shows the direction to the selected target when it is \
                        out of view.",
            },
        ],
    },
];

/// Every video filter the game offers, with what each does.
const FILTERS: [(Filter, &str, &str); 4] = [
    (
        Filter::Nearest,
        "Nearest",
        "Sharp square pixels: the original look.",
    ),
    (
        Filter::Linear,
        "Linear",
        "Smooth scaling; differs from Nearest only when the size is not a whole number.",
    ),
    (
        Filter::XbrzX2,
        "xBRZ 2x",
        "Edge-smoothing pixel-art upscaler, doubled before fitting the window.",
    ),
    (
        Filter::Scale2x,
        "Scale2x",
        "A fast edge-directed doubling that keeps edges sharper than xBRZ.",
    ),
];

/// All the filters at once, one per row, the chosen one marked.
fn filter_list(ui: &mut egui::Ui, current: &mut Filter) {
    for (filter, name, about) in FILTERS {
        let selected = *current == filter;
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
        let painter = ui.painter();
        if selected || response.hovered() {
            painter.rect_filled(
                rect,
                CornerRadius::same(8),
                if selected {
                    Color32::from_rgba_unmultiplied(113, 128, 0, 70)
                } else {
                    Color32::from_rgba_unmultiplied(120, 130, 30, 35)
                },
            );
        }
        let dot = pos2(rect.left() + 22.0, rect.center().y);
        painter.circle_stroke(dot, 9.0, Stroke::new(1.5_f32, CREAM));
        if selected {
            painter.circle_filled(dot, 5.0, VERIFIED);
        }
        painter.text(
            pos2(rect.left() + 46.0, rect.center().y),
            Align2::LEFT_CENTER,
            name,
            draw::font(19.0, draw::BOLD),
            CREAM,
        );
        painter.text(
            pos2(rect.left() + 176.0, rect.center().y),
            Align2::LEFT_CENTER,
            about,
            draw::font(17.0, draw::ITALIC),
            DIM,
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            *current = filter;
        }
    }
}

// --- About ---------------------------------------------------------------

impl App {
    fn about(&mut self, ui: &mut egui::Ui, panel: Rect) {
        draw::rounded_panel(
            ui.painter(),
            panel,
            16.0,
            Color32::from_rgba_unmultiplied(7, 18, 15, 236),
            Stroke::new(1.5_f32, Color32::from_rgb(215, 222, 210)),
        );
        let inner = panel.shrink2(vec2(30.0, 22.0));
        ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
            ui.label(RichText::new("About").font(draw::font(30.0, draw::BOLD)).color(CREAM));
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new(
                    "ffe-rs is a native Rust port of Frontier: First Encounters, checked against the JJFFE \
                     reconstruction of the original. It needs your own copy of the game: the launcher \
                     fingerprints the files and the game reads its content straight from them at startup.",
                ).color(CREAM));
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Saves and recordings: {}", self.root.display())).color(DIM));
                section(ui, "Game files");
                report(ui, self.game_report.as_ref(), self.busy(Job::Game));
                ui.horizontal(|ui| {
                    if ui.button("Open saves folder").clicked() {
                        open_folder(&self.saves);
                    }
                    let log = self.saves.join(GAME_LOG);
                    if ui.add_enabled(log.is_file(), egui::Button::new("Open last game log")).clicked() {
                        open_folder(&log);
                    }
                });
                section(ui, "Updates");
                let text = match &self.update {
                    UpdateState::Idle => "Updates are checked when you ask, or at start if enabled below.".to_owned(),
                    UpdateState::Checking(_) => "Checking GitHub for a newer release...".to_owned(),
                    UpdateState::UpToDate => format!("You have the latest release (v{}).", env!("CARGO_PKG_VERSION")),
                    UpdateState::Available(release) => format!("{} is available (you have v{}).", release.tag, env!("CARGO_PKG_VERSION")),
                    UpdateState::Installing(_, release) => format!("Installing {}...", release.tag),
                    UpdateState::Failed(error) => error.clone(),
                };
                ui.label(RichText::new(text).color(CREAM));
                ui.horizontal(|ui| {
                    let busy = matches!(self.update, UpdateState::Checking(_) | UpdateState::Installing(..));
                    if ui.add_enabled(!busy, egui::Button::new("Check for updates")).clicked() {
                        self.check_for_update();
                    }
                    if let UpdateState::Available(release) = &self.update {
                        let release = release.clone();
                        let installs = update::can_install(&self.install_dir) && release.asset_url.is_some();
                        if ui.button(if installs { "Install update" } else { "Open release page" }).clicked() {
                            self.update_action(release.clone());
                        }
                        if ui.button("Release notes").clicked() {
                            open_url(&release.page);
                        }
                    }
                });
                let before = self.launcher.check_updates;
                option_row(
                    ui,
                    "Check for updates at start",
                    "Asks GitHub for the latest release when the launcher opens. Nothing but the request is sent.",
                    |ui| {
                        toggle(ui, &mut self.launcher.check_updates);
                    },
                );
                if self.launcher.check_updates != before {
                    self.save_launcher();
                }
                section(ui, "Source code");
                ui.horizontal(|ui| {
                    ui.label(RichText::new(GITHUB_URL).color(CREAM));
                    if ui.button("Open on GitHub").clicked() {
                        open_url(GITHUB_URL);
                    }
                });
                section(ui, "Notes");
                ui.label(RichText::new(
                    "Never share the game files or recordings of copyrighted content. The \
                     launcher artwork is concept art, not original game assets.",
                ).color(DIM));
            });
        });
    }
}

// --- Widgets -------------------------------------------------------------

fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(12.0);
    ui.label(
        RichText::new(text.to_uppercase())
            .font(draw::font(16.0, draw::BOLD))
            .color(CYAN)
            .extra_letter_spacing(1.5),
    );
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 169, 230, 90));
    ui.add_space(2.0);
}

fn option_row(ui: &mut egui::Ui, title: &str, about: &str, control: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 420.0).max(300.0));
            ui.label(
                RichText::new(title)
                    .font(draw::font(19.0, draw::BOLD))
                    .color(CREAM),
            );
            ui.label(RichText::new(about).color(DIM).italics());
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
    });
    ui.add_space(4.0);
}

fn pill(label: &str, selected: bool) -> egui::Button<'static> {
    let text = RichText::new(label.to_owned())
        .font(draw::font(
            17.0,
            if selected { draw::BOLD } else { draw::REGULAR },
        ))
        .color(CREAM);
    egui::Button::new(text)
        .fill(if selected {
            OLIVE
        } else {
            Color32::from_rgb(16, 30, 24)
        })
        .stroke(Stroke::new(
            1.0_f32,
            if selected {
                CREAM
            } else {
                Color32::from_rgba_unmultiplied(215, 222, 210, 80)
            },
        ))
        .corner_radius(CornerRadius::same(8))
}

fn choice<T: PartialEq + Copy>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)]) {
    // Right-to-left layout: add in reverse so they read left to right.
    for (option, label) in options.iter().rev() {
        if ui.add(pill(label, *value == *option)).clicked() {
            *value = *option;
        }
    }
}

fn combo(ui: &mut egui::Ui, id: &str, index: &mut usize, labels: &[String]) -> bool {
    let before = *index;
    *index = (*index).min(labels.len().saturating_sub(1));
    egui::ComboBox::from_id_salt(id)
        .width(220.0)
        .selected_text(labels.get(*index).cloned().unwrap_or_default())
        .show_ui(ui, |ui| {
            for (i, label) in labels.iter().enumerate() {
                ui.selectable_value(index, i, label);
            }
        });
    *index != before
}

fn toggle(ui: &mut egui::Ui, on: &mut bool) -> bool {
    let (rect, mut response) = ui.allocate_exact_size(vec2(52.0, 26.0), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(response.id, *on);
    let off = Color32::from_rgb(30, 44, 38);
    let fill = off.lerp_to_gamma(OLIVE, t);
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(13),
        fill,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(215, 222, 210, 120)),
        egui::StrokeKind::Inside,
    );
    let x = egui::lerp(rect.left() + 13.0..=rect.right() - 13.0, t);
    painter.circle_filled(pos2(x, rect.center().y), 9.5, CREAM);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .changed()
}

fn toggle_grid(ui: &mut egui::Ui, id: &str, values: &mut [bool], labels: &[&str]) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([60.0, 12.0])
        .show(ui, |ui| {
            for (i, (value, label)) in values.iter_mut().zip(labels).enumerate() {
                ui.horizontal(|ui| {
                    toggle(ui, value);
                    ui.label(RichText::new(*label).color(CREAM));
                });
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

fn report(ui: &mut egui::Ui, report: Option<&Report>, busy: bool) {
    if busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new("Working...").color(DIM));
        });
        return;
    }
    let Some(report) = report else {
        ui.label(RichText::new("Not checked yet.").color(DIM));
        return;
    };
    ui.label(
        RichText::new(&report.summary)
            .font(draw::font(19.0, draw::BOLD))
            .color(level_colour(report.level)),
    );
    for line in &report.lines {
        ui.horizontal_wrapped(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
            draw::icon(
                ui.painter(),
                match line.level {
                    Level::Good => Icon::Verified,
                    Level::Warning => Icon::Warning,
                    Level::Bad => Icon::Failed,
                },
                rect,
                level_colour(line.level),
            );
            ui.label(RichText::new(&line.text).color(CREAM));
        });
    }
}

// --- Platform ------------------------------------------------------------

fn open_folder(folder: &Path) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = Command::new(program).arg(folder).spawn();
}

/// Open a web address in the default browser (the same helpers open folders).
fn open_url(url: &str) {
    open_folder(Path::new(url));
}

#[cfg(any(windows, target_os = "macos"))]
fn pick_folder(title: &str, start: &Path) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    if start.is_dir() {
        dialog = dialog.set_directory(start);
    }
    dialog.pick_folder()
}

#[cfg(not(any(windows, target_os = "macos")))]
fn pick_folder(title: &str, start: &Path) -> Option<PathBuf> {
    // zenity (GNOME) or kdialog (KDE); without either, type the path in
    // Settings > Files & start.
    let start = if start.is_dir() {
        start.display().to_string()
    } else {
        String::from(".")
    };
    let attempts: [(&str, Vec<String>); 2] = [
        (
            "zenity",
            vec![
                "--file-selection".into(),
                "--directory".into(),
                format!("--title={title}"),
                format!("--filename={start}/"),
            ],
        ),
        (
            "kdialog",
            vec![
                "--getexistingdirectory".into(),
                start.clone(),
                "--title".into(),
                title.into(),
            ],
        ),
    ];
    for (program, args) in attempts {
        if let Ok(output) = Command::new(program).args(&args).output() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            return (output.status.success() && !path.is_empty()).then(|| PathBuf::from(path));
        }
    }
    None
}

struct Status {
    icon: Icon,
    text: String,
    colour: Color32,
    busy: bool,
}

impl App {
    fn game_status(&self) -> Status {
        let busy = |text: &str| Status {
            icon: Icon::Circle,
            text: text.to_owned(),
            colour: CREAM,
            busy: true,
        };
        if self.busy(Job::Game) {
            return busy("Verifying...");
        }
        match &self.game_report {
            None if !self.launcher.game.is_dir() => Status {
                icon: Icon::Circle,
                text: "Not configured".to_owned(),
                colour: CREAM,
                busy: false,
            },
            None => busy("Verifying..."),
            Some(report) if report.level == Level::Bad => Status {
                icon: Icon::Failed,
                text: if self.launcher.game.join("FIRSTENC.EXE").is_file() {
                    "Not recognised"
                } else {
                    "Not found"
                }
                .to_owned(),
                colour: FAILED,
                busy: false,
            },
            Some(report)
                if report
                    .lines
                    .iter()
                    .any(|line| line.text.starts_with("Shareware identified;")) =>
            {
                Status {
                    icon: Icon::Warning,
                    text: "Experimental".to_owned(),
                    colour: WARNING,
                    busy: false,
                }
            }
            Some(_) => Status {
                icon: Icon::Verified,
                text: "Verified".to_owned(),
                colour: VERIFIED,
                busy: false,
            },
        }
    }

    fn video_status(&self) -> Status {
        if self.videos.found() {
            Status {
                icon: Icon::Verified,
                text: "Verified".to_owned(),
                colour: VERIFIED,
                busy: false,
            }
        } else {
            Status {
                icon: Icon::Circle,
                text: "Not found".to_owned(),
                colour: DIM,
                busy: false,
            }
        }
    }

    fn play(&mut self, ui: &mut egui::Ui, rect: &dyn Fn(f32, f32, f32, f32) -> Rect) {
        let status = self.game_status();
        let running = self.child.is_some();
        // The icon and the line under the title say which release was found.
        let release = self
            .game_report
            .as_ref()
            .and_then(|report| report.release.clone());
        let (game_icon, game_subtitle) = match &release {
            Some(release) => (
                match release.kind {
                    Kind::CdRom => Icon::Disc,
                    Kind::Floppy => Icon::Floppy,
                    Kind::Shareware => Icon::Shareware,
                },
                release.text.clone(),
            ),
            None => (Icon::Disc, "Original game files (required)".to_owned()),
        };
        ui.add_enabled_ui(!running, |ui| {
            let action = self.file_row(
                ui,
                rect(309.0, 495.0, 1515.0, 567.0),
                &RowSpec {
                    icon: game_icon,
                    title: "Frontier: First Encounters (Elite 3)",
                    subtitle: &game_subtitle,
                    status: &status,
                    status_opens_details: true,
                    folder: &self.launcher.game.clone(),
                    id: "game",
                    browse_title: "Choose the folder with FIRSTENC.EXE",
                },
            );
            match action {
                RowAction::Status => self.page = Page::About,
                RowAction::Browse(path) => self.verify_game_folder(path),
                RowAction::None => {}
            }
            // The game's videos: the game folder's DATA folder when there is
            // one, else the folder the player picks.
            let video_status = self.video_status();
            let video_subtitle = match (&self.videos.folder, self.videos.found()) {
                (Some(folder), true) => format!(
                    "{} AVI files found in {}",
                    self.videos.count,
                    folder.file_name().map_or_else(
                        || folder.display().to_string(),
                        |n| n.to_string_lossy().into_owned()
                    )
                ),
                (Some(_), false) => "No AVI files in that folder".to_owned(),
                (None, _) => "Optional: Browse to the CD-ROM's DATA folder".to_owned(),
            };
            let action = self.file_row(
                ui,
                rect(309.0, 576.0, 1515.0, 648.0),
                &RowSpec {
                    icon: Icon::Film,
                    title: "Game videos (FMV)",
                    subtitle: &video_subtitle,
                    status: &video_status,
                    status_opens_details: false,
                    folder: &self.videos.folder.clone().unwrap_or_default(),
                    id: "videos",
                    browse_title:
                        "Choose the folder with the game's AVI videos (the CD-ROM's DATA folder)",
                },
            );
            if let RowAction::Browse(path) = action {
                self.choose_videos(path);
            }
        });
        let panel = rect(309.0, 658.0, 1515.0, 826.0);
        draw::rounded_panel(
            ui.painter(),
            panel,
            14.0,
            PANEL,
            Stroke::new(1.5_f32, CREAM),
        );
        ui.scope_builder(
            UiBuilder::new().max_rect(panel.shrink2(vec2(28.0, 10.0))),
            |ui| {
                ui.add_enabled_ui(!running, |ui| {
                    let before = self.launcher.clone();
                    let mut start = config::STARTS
                        .iter()
                        .position(|(code, _, _)| *code == self.launcher.start)
                        .unwrap_or(0);
                    let labels: Vec<String> = config::STARTS
                        .iter()
                        .map(|(_, name, _)| (*name).to_owned())
                        .collect();
                    option_row(ui, "Starting location", config::STARTS[start].2, |ui| {
                        combo(ui, "start-location", &mut start, &labels);
                    });
                    self.launcher.start = config::STARTS[start].0;
                    option_row(ui, "Skip intro", "Go straight to the game.", |ui| {
                        toggle(ui, &mut self.launcher.skip_intro);
                    });
                    if self.launcher != before {
                        self.save_launcher();
                    }
                });
            },
        );
        let ready = self.game_ok() && !self.busy(Job::Game) && !running;
        let button = rect(309.0, 840.0, 1515.0, 946.0);
        let response = ui.interact(
            button,
            ui.id().with("launch"),
            if ready {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        draw::launch_button(
            ui.painter(),
            button,
            if running { "GAME RUNNING" } else { "LAUNCH" },
            ready,
            response.hovered() && ready,
            response.is_pointer_button_down_on(),
        );
        if ready
            && response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
        {
            self.launch();
        }
    }

    /// One of the Play page's rounded rows: icon, title and subtitle, a status
    /// (click for details when `status_opens_details`), the open-folder button
    /// and Browse.
    fn file_row(&self, ui: &mut egui::Ui, row: Rect, spec: &RowSpec) -> RowAction {
        let painter = ui.painter().clone();
        draw::rounded_panel(
            &painter,
            row,
            14.0,
            PANEL,
            Stroke::new(1.5_f32, Color32::from_rgb(215, 222, 210)),
        );
        let mid = row.center().y;
        draw::icon(
            &painter,
            spec.icon,
            Rect::from_center_size(pos2(row.left() + 69.0, mid), Vec2::splat(48.0)),
            CREAM,
        );
        painter.text(
            pos2(row.left() + 132.0, mid - 13.0),
            Align2::LEFT_CENTER,
            spec.title,
            draw::font(25.0, draw::BOLD),
            CREAM,
        );
        painter.text(
            pos2(row.left() + 132.0, mid + 14.0),
            Align2::LEFT_CENTER,
            spec.subtitle,
            draw::font(20.0, draw::ITALIC),
            CREAM,
        );
        let id = spec.id;
        let status = spec.status;
        let folder = spec.folder;
        let mut action = RowAction::None;
        // Status: icon and text.
        let status_rect = Rect::from_min_max(
            pos2(row.left() + 722.0, mid - 22.0),
            pos2(row.left() + 930.0, mid + 22.0),
        );
        let response = ui.interact(
            status_rect,
            ui.id().with((id, "status")),
            if spec.status_opens_details {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let icon_rect = Rect::from_center_size(pos2(row.left() + 743.0, mid), Vec2::splat(34.0));
        if status.busy {
            let t = ui.input(|i| i.time) as f32;
            draw::spinner(&painter, icon_rect, t, CYAN);
            ui.ctx().request_repaint();
        } else {
            draw::icon(&painter, status.icon, icon_rect, status.colour);
        }
        painter.text(
            pos2(row.left() + 775.0, mid),
            Align2::LEFT_CENTER,
            &status.text,
            draw::font(21.0, draw::REGULAR),
            status.colour,
        );
        if spec.status_opens_details
            && !status.busy
            && response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Details")
                .clicked()
        {
            action = RowAction::Status;
        }
        // Open the folder, and Browse.
        let open = Rect::from_min_max(
            pos2(row.left() + 946.0, mid - 21.0),
            pos2(row.left() + 1012.0, mid + 21.0),
        );
        let response = ui.interact(open, ui.id().with((id, "open")), Sense::click());
        draw::small_button(&painter, open, response.hovered());
        draw::icon(
            &painter,
            Icon::Folder,
            Rect::from_center_size(open.center(), Vec2::splat(30.0)),
            CREAM,
        );
        let tooltip = if folder.as_os_str().is_empty() {
            "No folder chosen".to_owned()
        } else {
            format!("Open {}", folder.display())
        };
        if response.on_hover_text(tooltip).clicked() && folder.is_dir() {
            open_folder(folder);
        }
        let browse = Rect::from_min_max(
            pos2(row.left() + 1036.0, mid - 21.0),
            pos2(row.left() + 1181.0, mid + 21.0),
        );
        let response = ui.interact(browse, ui.id().with((id, "browse")), Sense::click());
        draw::small_button(&painter, browse, response.hovered());
        painter.text(
            browse.center(),
            Align2::CENTER_CENTER,
            "Browse...",
            draw::font(21.0, draw::REGULAR),
            CREAM,
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            if let Some(path) = pick_folder(spec.browse_title, folder) {
                action = RowAction::Browse(path);
            }
        }
        action
    }
}

/// What a Play-page row shows.
struct RowSpec<'a> {
    icon: Icon,
    title: &'a str,
    subtitle: &'a str,
    status: &'a Status,
    status_opens_details: bool,
    folder: &'a Path,
    id: &'a str,
    browse_title: &'a str,
}

enum RowAction {
    None,
    Status,
    Browse(PathBuf),
}

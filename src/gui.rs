//! egui window, "instrument panel" layout: header with mark, path row, component
//! tiles, one Install button, progress, log.

use crate::diagnose;
use crate::feeder_cfg::{self, FeederKnobs};
use crate::game::{self, GameStatus};
use crate::hotkeys::{self, GameHotkeys, KeyChord};
use crate::i18n::{fmt_n, fmt_vc, Language, T};
use crate::installer::{self, Engine, StepState};
use crate::library::{self, Game, Store};
use crate::logo;
use crate::net;
use crate::quality_preset::QualityChoice;
use crate::renodx;
use crate::reshade_ini;
use crate::settings::Settings;
use crate::text;
use crate::theme::{self as t};
use crate::ui_components as ui_c;
use crate::update;
use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, StrokeKind, Vec2,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Clone)]
enum UpdateState {
    Idle,
    Available(update::Available),
    Downloading(u8, String),
    Restarting,
    Failed(String),
}

enum Msg {
    Progress(u8, String),
    Log(LogLine),
    Finished(Result<String, String>),
}

#[derive(Clone)]
enum LogLine {
    Step(String),
    Ok(String),
    Fail(String),
    Plain(String),
}

pub struct App {
    exe_text: String,
    status: Option<Result<GameStatus, String>>,
    running: bool,
    progress: u8,
    progress_msg: String,
    log: Vec<LogLine>,
    rx: Option<Receiver<Msg>>,
    confirm_remove: bool,
    last_error: Option<String>,
    candidates: Vec<PathBuf>,
    resolved_exe: Option<PathBuf>,
    /// When the folder is a multi-game collection (MELE, …), which exes to Install/Remove.
    collection_selected: HashSet<PathBuf>,
    engine: Engine,
    update: UpdateState,
    update_rx: Option<Receiver<UpdateState>>,
    skipped_version: String,
    /// A card whose install just finished and whose state has to be re-read.
    pending_refresh: Option<usize>,
    /// The card being installed from the Games page, so the progress is shown
    /// where the user started it instead of throwing them onto another page.
    updating: Option<usize>,
    /// One refreshed card, after its install finished.
    meta_one_rx: Option<Receiver<(usize, GameMeta)>>,
    /// The user pressed "Check for updates": the answer is worth showing even
    /// when it is "nothing new", which a background check never says.
    checked_manually: bool,
    /// When the last update check ran. The check used to happen once, at
    /// startup, so a release published while the window was open was never
    /// noticed -- the tool sat there saying it was current for days.
    last_update_check: std::time::Instant,
    /// "Also install the RenoDX HDR mod" checkbox.
    renodx_on: bool,
    /// OptiScaler route: fraction of the frame the DLSS 5 model works at.
    /// Its cost falls with the square of this, so it is the biggest fps lever
    /// on that route. 1.0 = full size.
    working_scale: f32,
    /// ReShade engine: run the experimental neural-upstream consumer instead of
    /// the stable RenoDX DLSS 5 add-on.
    upstream_on: bool,
    /// neural-upstream strength preset to write into ReShade.ini before the
    /// game starts (#68). 3 = Reference, the add-on's own default.
    upstream_preset: u8,
    /// OptiScaler route: install wilsjo2's pre-SR multipass fork instead of
    /// Dagherbou's build (#72).
    opti_presr: bool,
    /// ReShade route: pin the classic DLSS 5 add-on build, which the Feeder's
    /// host measured to work on NVIDIA 616.64 where the current one faults (#69).
    renodx_classic: bool,
    /// OptiScaler route, RTX 40 only: the fork's built-in MFG unlock (#83).
    ada_mfg: bool,
    renodx: RenodxLookup,
    renodx_rx: Option<Receiver<RenodxLookup>>,
    /// Exe the current lookup belongs to, so a refresh does not re-fetch.
    renodx_for: Option<PathBuf>,
    page: Page,
    games: Vec<Game>,
    scanning: bool,
    scan_rx: Option<Receiver<Vec<Game>>>,
    /// Per game: decoded poster texture, or `None` once decoding failed.
    posters: HashMap<usize, Option<egui::TextureHandle>>,
    poster_rx: Option<Receiver<(usize, Option<egui::ColorImage>)>>,
    meta: HashMap<usize, GameMeta>,
    meta_rx: Option<Receiver<(usize, GameMeta)>>,
    search: String,
    /// Official store marks (Simple Icons, CC0), white on transparent, tinted at paint time.
    store_icons: HashMap<Store, egui::TextureHandle>,
    #[allow(dead_code)] // kept for a future icon-capable support button
    kofi_icon: Option<egui::TextureHandle>,
    /// Global defaults from settings.json.
    settings: Settings,
    /// Offline Feeder knobs for the selected game (None = not loaded / not installed).
    knobs: Option<FeederKnobs>,
    knobs_err: Option<String>,
    knobs_dirty: bool,
    /// Per-game hotkeys (ReShade / NR add-on / OptiScaler).
    hotkeys: Option<GameHotkeys>,
    hotkeys_err: Option<String>,
    hotkeys_dirty: bool,
    /// Which binding is waiting for a key press (`None` = not capturing).
    hotkey_capture: Option<HotkeySlot>,
    /// Collapsed install log shows a one-line summary.
    log_expanded: bool,
    /// First-run tip dismissed (also persisted in eframe storage / settings).
    tip_dismissed: bool,
}

pub const KOFI_URL: &str = "https://ko-fi.com/kindiboy";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Games,
    Setup,
    Settings,
    About,
}

/// Which hotkey row is listening for a press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotkeySlot {
    ReshadeOverlay,
    NrToggle,
    NrScreenshot,
    OptiMenu,
}

/// What a poster card asked for this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CardAction {
    #[default]
    None,
    /// Left click: open the game on the Setup page.
    Open,
    /// Install / Update / Re-install from the card (or context menu).
    Update,
    /// Right click ▸ Forget: drop a hand-added game from the list.
    Forget,
}

/// What the tool knows about an installed game without touching it.
#[derive(Debug, Clone)]
struct GameMeta {
    api: &'static str,
    /// Game ships its own DLSS (Mode::Native).
    has_dlss: bool,
    /// Feeder / Native / Opti path label.
    engine_path: &'static str,
    addon: bool,
    ready: bool,
    /// This tool installed into that folder.
    installed: bool,
    /// Components this tool placed that upstream has since moved past.
    stale: Vec<String>,
    rt_likely: bool,
    unreal_likely: bool,
    unity_likely: bool,
    re_engine: bool,
    shaders_missing: bool,
    wrong_folder: Option<String>,
    /// Canonical Shipping (or best) exe.
    exe: PathBuf,
}

fn meta_from_status(st: &GameStatus, latest: &installer::Latest) -> GameMeta {
    let dir = st.game_dir();
    GameMeta {
        api: match st.api {
            game::Api::Vulkan => "Vulkan",
            game::Api::Dx9 => "DirectX 9",
            game::Api::Dx10 => "DirectX 10",
            game::Api::Dx11 => "DirectX 11",
            game::Api::Dx12 => "DirectX 12",
            game::Api::Unknown => "DirectX 12?",
        },
        has_dlss: st.mode == game::Mode::Native,
        engine_path: if st.opti {
            "Opti"
        } else if st.mode == game::Mode::Native {
            "Native"
        } else {
            "Feeder"
        },
        addon: st.dlss5_addon || st.opti,
        ready: st.complete(),
        installed: game::installed_by_tool(dir),
        stale: installer::stale_components(dir, latest),
        rt_likely: st.rt_likely,
        unreal_likely: st.unreal_likely,
        unity_likely: st.unity_likely,
        re_engine: st.re_engine,
        shaders_missing: game::shaders_missing(dir),
        wrong_folder: game::install_folder_mismatch(&st.exe),
        exe: st.exe.clone(),
    }
}

#[derive(Debug, Clone)]
enum RenodxLookup {
    Idle,
    Pending,
    Found(renodx::Mod),
    NotFound,
    Failed(String),
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        t::install(&cc.egui_ctx);
        logo::set_taskbar_icon(cc);
        let exe_text = cc
            .storage
            .and_then(|s| s.get_string("exe"))
            .unwrap_or_default();
        let mut app = App {
            exe_text,
            status: None,
            running: false,
            progress: 0,
            progress_msg: String::new(),
            log: Vec::new(),
            rx: None,
            confirm_remove: false,
            last_error: None,
            candidates: Vec::new(),
            resolved_exe: None,
            collection_selected: HashSet::new(),
            engine: Engine::default(),
            update: UpdateState::Idle,
            update_rx: None,
            renodx_on: false,
            working_scale: 1.0,
            upstream_on: false,
            upstream_preset: 3,
            opti_presr: false,
            renodx_classic: false,
            ada_mfg: false,
            renodx: RenodxLookup::Idle,
            renodx_rx: None,
            renodx_for: None,
            page: Page::Games,
            games: Vec::new(),
            scanning: false,
            scan_rx: None,
            posters: HashMap::new(),
            poster_rx: None,
            meta: HashMap::new(),
            meta_rx: None,
            search: String::new(),
            store_icons: HashMap::new(),
            kofi_icon: None,
            updating: None,
            pending_refresh: None,
            meta_one_rx: None,
            checked_manually: false,
            last_update_check: std::time::Instant::now(),
            skipped_version: cc
                .storage
                .and_then(|s| s.get_string("skip_version"))
                .unwrap_or_default(),
            settings: Settings::load(),
            knobs: None,
            knobs_err: None,
            knobs_dirty: false,
            hotkeys: None,
            hotkeys_err: None,
            hotkeys_dirty: false,
            hotkey_capture: None,
            log_expanded: true,
            tip_dismissed: cc
                .storage
                .and_then(|s| s.get_string("tip_dismissed"))
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        };
        app.refresh();
        if app.resolved_exe.is_some() {
            app.page = Page::Setup;
        }
        if update::ENABLED {
            app.start_update_check();
        }
        app.start_scan(&cc.egui_ctx);
        app
    }

    fn exe(&self) -> Option<PathBuf> {
        self.resolved_exe.clone()
    }

    fn input_path(&self) -> PathBuf {
        PathBuf::from(self.exe_text.trim().trim_matches('"'))
    }

    /// Text box accepts an exe or a game folder; a folder is resolved to its game exe.
    fn refresh(&mut self) {
        let input = self.input_path();
        let prev = self.resolved_exe.take();
        let prev_sel = std::mem::take(&mut self.collection_selected);
        self.candidates.clear();
        if input.as_os_str().is_empty() {
            self.status = None;
            return;
        }
        match game::resolve_target(&input) {
            Ok((exe, cands)) => {
                self.resolved_exe = Some(match prev {
                    Some(p) if cands.len() > 1 && cands.contains(&p) => p,
                    _ => exe,
                });
                self.candidates = cands;
                let members = game::collection_members(&self.candidates);
                if members.len() >= 2 {
                    // Keep prior ticks when still valid; otherwise select every sub-game.
                    let kept: HashSet<PathBuf> = prev_sel
                        .into_iter()
                        .filter(|p| members.iter().any(|m| m == p))
                        .collect();
                    self.collection_selected = if kept.is_empty() {
                        members.into_iter().collect()
                    } else {
                        kept
                    };
                } else {
                    self.collection_selected.clear();
                }
            }
            Err(e) => {
                self.status = Some(Err(format!("{e:#}")));
                return;
            }
        }
        self.inspect_resolved();
    }

    /// Targets for Install / Remove: collection selection, or the single resolved exe.
    fn install_targets(&self) -> Vec<PathBuf> {
        let members = game::collection_members(&self.candidates);
        if members.len() >= 2 {
            let mut v: Vec<PathBuf> = members
                .into_iter()
                .filter(|p| self.collection_selected.contains(p))
                .collect();
            if v.is_empty() {
                if let Some(exe) = self.resolved_exe.clone() {
                    v.push(exe);
                }
            }
            v
        } else {
            self.exe().into_iter().collect()
        }
    }

    fn inspect_resolved(&mut self) {
        self.status = self
            .resolved_exe
            .as_ref()
            .map(|p| game::inspect(p).map_err(|e| format!("{e:#}")));
        if self.resolved_exe != self.renodx_for {
            self.renodx_for = self.resolved_exe.clone();
            self.renodx_on = false;
            self.start_renodx_lookup();
        }
        self.reload_knobs_and_perf();
        self.reload_hotkeys();
    }

    fn reload_knobs_and_perf(&mut self) {
        self.knobs = None;
        self.knobs_err = None;
        self.knobs_dirty = false;
        let Some(exe) = self.resolved_exe.clone() else {
            return;
        };
        let Some(dir) = exe.parent().map(|p| p.to_path_buf()) else {
            return;
        };
        match feeder_cfg::load(&dir) {
            Ok(k) => {
                self.knobs = Some(k);
            }
            Err(e) => self.knobs_err = Some(format!("{e:#}")),
        }
    }

    fn reload_hotkeys(&mut self) {
        self.hotkeys = None;
        self.hotkeys_err = None;
        self.hotkeys_dirty = false;
        self.hotkey_capture = None;
        let Some(Ok(st)) = self.status.as_ref() else {
            return;
        };
        let game_dir = st.game_dir().to_path_buf();
        let consumer = st.consumer_dir();
        match hotkeys::load(&game_dir, &consumer) {
            Ok(h) => {
                if h.has_reshade || h.has_opti {
                    self.hotkeys = Some(h);
                } else {
                    self.hotkeys_err = Some(self.tr(T::InstallHotkeysFirst).into());
                }
            }
            Err(e) => self.hotkeys_err = Some(format!("{e:#}")),
        }
    }

    fn apply_settings_to_game(&mut self) {
        let Some(exe) = self.exe() else { return };
        let Some(dir) = exe.parent().map(|p| p.to_path_buf()) else {
            return;
        };
        let s = &self.settings;
        let mut k = self.knobs.clone().unwrap_or_default();
        if let Some(v) = s.knobs.work_resolution {
            k.work_resolution = v;
        }
        if let Some(v) = s.knobs.ofa_enabled {
            k.ofa_enabled = v;
        }
        if let Some(v) = s.knobs.ofa_grid {
            k.ofa_grid = v;
        }
        if let Some(v) = s.knobs.reset_mode {
            k.reset_mode = v;
        }
        if let Some(v) = s.knobs.light_stab {
            k.light_stab = v;
        }
        if let Some(v) = s.knobs.engine_velocity {
            k.engine_velocity = v;
        }
        k.evaluate_stride = s.overlay.evaluate_stride;
        k.log_detail = s.overlay.log_detail;
        match feeder_cfg::save(&dir, &k) {
            Ok(()) => {
                self.knobs = Some(k);
                self.knobs_err = None;
                self.knobs_dirty = false;
                self.log
                    .push(LogLine::Ok("Applied Settings defaults to this game".into()));
            }
            Err(e) => self.knobs_err = Some(format!("{e:#}")),
        }
    }

    fn start_renodx_lookup(&mut self) {
        let Some(exe) = self.exe() else {
            self.renodx = RenodxLookup::Idle;
            return;
        };
        let (tx, rx) = channel::<RenodxLookup>();
        self.renodx_rx = Some(rx);
        self.renodx = RenodxLookup::Pending;
        thread::spawn(move || {
            let r = match net::client().and_then(|c| renodx::lookup(&c, &exe)) {
                Ok(Some(m)) => RenodxLookup::Found(m),
                Ok(None) => RenodxLookup::NotFound,
                Err(e) => RenodxLookup::Failed(format!("{e:#}")),
            };
            let _ = tx.send(r);
        });
    }

    fn pump_renodx(&mut self) {
        let Some(rx) = &self.renodx_rx else { return };
        if let Ok(r) = rx.try_recv() {
            self.renodx = r;
            self.renodx_rx = None;
        }
    }

    fn start(&mut self, remove: Option<bool>) {
        let targets = self.install_targets();
        if targets.is_empty() {
            return;
        }
        let root = self.input_path();
        let labels: Vec<String> = targets.iter().map(|p| game::exe_label(&root, p)).collect();
        let engine = self.engine;
        let with_renodx = self.renodx_on;
        let upstream = self.upstream_on;
        std::env::set_var(
            installer::WORKING_SCALE_ENV,
            format!("{:.2}", self.working_scale),
        );
        if self.opti_presr {
            std::env::set_var(installer::OPTI_SOURCE_ENV, "presr");
        } else {
            std::env::remove_var(installer::OPTI_SOURCE_ENV);
        }
        if self.renodx_classic {
            std::env::set_var(installer::RENODX_TAG_ENV, installer::RENODX_CLASSIC_TAG);
        } else {
            std::env::remove_var(installer::RENODX_TAG_ENV);
        }
        if self.ada_mfg {
            std::env::set_var(installer::ADA_MFG_ENV, "1");
        } else {
            std::env::remove_var(installer::ADA_MFG_ENV);
        }
        std::env::set_var(
            installer::UPSTREAM_PRESET_ENV,
            if upstream {
                self.upstream_preset.to_string()
            } else {
                "0".to_owned()
            },
        );
        // Bindings from the Hotkeys panel, or stock defaults so a multi-game
        // Install still seeds every sub-game's ReShade.ini.
        let hotkey_template = self.hotkeys.clone().unwrap_or_default();
        let (tx, rx): (Sender<Msg>, Receiver<Msg>) = channel();
        self.rx = Some(rx);
        self.running = true;
        self.progress = 0;
        self.progress_msg.clear();
        self.log.clear();
        self.last_error = None;
        thread::spawn(move || {
            let n = targets.len();
            let mut ok_names = Vec::new();
            let mut err_names = Vec::new();
            for (ti, exe) in targets.iter().enumerate() {
                let label = labels.get(ti).cloned().unwrap_or_else(|| {
                    exe.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| exe.display().to_string())
                });
                let _ = tx.send(Msg::Log(LogLine::Step(format!(
                    "—— {} ({}/{}) ——",
                    label,
                    ti + 1,
                    n
                ))));
                let is_install = remove.is_none();
                let out = if let Some(everything) = remove {
                    let res = if everything {
                        installer::uninstall_all(exe).map(|(mut r, kept)| {
                            r.extend(kept);
                            r
                        })
                    } else {
                        installer::uninstall(exe)
                    };
                    res.map(|r| {
                        format!(
                            "Removed from {label}: {}",
                            if r.is_empty() {
                                "nothing".into()
                            } else {
                                r.join(", ")
                            }
                        )
                    })
                    .map_err(|e| format!("{label}: {e:#}"))
                } else {
                    let p_tx = tx.clone();
                    let s_tx = tx.clone();
                    let label_p = label.clone();
                    let label_s = label.clone();
                    let base = ((ti as f32) / (n as f32) * 100.0) as u8;
                    let span = (100.0 / n as f32).max(1.0) as u8;
                    installer::run_all(
                        exe,
                        engine,
                        with_renodx,
                        upstream,
                        &move |pct, msg| {
                            let scaled =
                                base.saturating_add(((pct as u16 * span as u16) / 100) as u8);
                            let _ = p_tx
                                .send(Msg::Progress(scaled.min(99), format!("[{label_p}] {msg}")));
                        },
                        &move |i, steps, name, state, detail| {
                            let line = match state {
                                StepState::Start => {
                                    LogLine::Step(format!("[{label_s}] [{}/{steps}] {name}", i + 1))
                                }
                                StepState::Done => LogLine::Ok(format!("[{label_s}] ok: {detail}")),
                                StepState::Error => {
                                    LogLine::Fail(format!("[{label_s}] FAILED: {detail}"))
                                }
                            };
                            let _ = s_tx.send(Msg::Log(line));
                        },
                    )
                    .map(|_| {
                        if engine == Engine::Opti {
                            format!(
                                "{label}: Insert → OptiScaler overlay → enable Neural Rendering."
                            )
                        } else {
                            format!("{label}: Home → Add-ons → DLSS 5 Neural Rendering → enable.")
                        }
                    })
                    .map_err(|e| format!("{label}: {e:#}"))
                };
                match out {
                    Ok(msg) => {
                        ok_names.push(label.clone());
                        let _ = tx.send(Msg::Log(LogLine::Ok(msg)));
                        // Same hotkey bindings into every installed sub-game.
                        if is_install {
                            match hotkeys::save_for_exe(&hotkey_template, exe) {
                                Ok(()) => {
                                    let _ = tx.send(Msg::Log(LogLine::Ok(format!(
                                        "[{label}] hotkeys written"
                                    ))));
                                }
                                Err(e) => {
                                    let _ = tx.send(Msg::Log(LogLine::Fail(format!(
                                        "[{label}] hotkeys: {e:#}"
                                    ))));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        err_names.push(label);
                        let _ = tx.send(Msg::Log(LogLine::Fail(e)));
                    }
                }
            }
            let summary = if err_names.is_empty() {
                Ok(if n == 1 {
                    ok_names.pop().unwrap_or_else(|| "Done.".into())
                } else {
                    format!(
                        "Done for {} game(s): {}. Enable DLSS 5 in each title's overlay.",
                        ok_names.len(),
                        ok_names.join(", ")
                    )
                })
            } else if ok_names.is_empty() {
                Err(format!("Failed: {}", err_names.join("; ")))
            } else {
                Err(format!(
                    "Partial: ok [{}]; failed [{}]",
                    ok_names.join(", "),
                    err_names.join(", ")
                ))
            };
            let _ = tx.send(Msg::Finished(summary));
        });
    }

    fn start_scan(&mut self, ctx: &egui::Context) {
        let (tx, rx) = channel::<Vec<Game>>();
        self.scan_rx = Some(rx);
        self.scanning = true;
        self.games.clear();
        self.posters.clear();
        self.meta.clear();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(library::scan());
            ctx.request_repaint();
        });
    }

    /// Decode posters and inspect each game in the background, oldest last.
    fn start_game_details(&mut self, ctx: &egui::Context) {
        let games = self.games.clone();
        let (ptx, prx) = channel();
        let (mtx, mrx) = channel();
        self.poster_rx = Some(prx);
        self.meta_rx = Some(mrx);
        let ctx2 = ctx.clone();
        let g2 = games.clone();
        thread::spawn(move || {
            // One lookup for the whole scan; every game is then compared
            // against it by reading the markers this tool wrote.
            let latest = net::client()
                .map(|c| installer::Latest::fetch(&c))
                .unwrap_or_default();
            for (i, g) in g2.iter().enumerate() {
                let meta = game::resolve_target(&g.dir)
                    .and_then(|(exe, _)| game::inspect(&exe))
                    .ok()
                    .map(|st| meta_from_status(&st, &latest));
                if let Some(m) = meta {
                    let _ = mtx.send((i, m));
                }
                if i % 8 == 7 {
                    ctx2.request_repaint();
                }
            }
            ctx2.request_repaint();
        });
        let ctx3 = ctx.clone();
        thread::spawn(move || {
            let client = net::client().ok();
            for (i, g) in games.iter().enumerate() {
                let img = client
                    .as_ref()
                    .and_then(|c| library::poster_rgba(c, &g.poster))
                    .map(|rgba| {
                        // Cards are 150 px wide; 300x450 keeps 2x for HiDPI and saves VRAM.
                        let (w, h) = (rgba.width(), rgba.height());
                        let scale = (300.0 / w as f32).min(450.0 / h as f32).min(1.0);
                        let (nw, nh) = (
                            ((w as f32 * scale) as u32).max(1),
                            ((h as f32 * scale) as u32).max(1),
                        );
                        let small = image::imageops::resize(
                            &rgba,
                            nw,
                            nh,
                            image::imageops::FilterType::Triangle,
                        );
                        egui::ColorImage::from_rgba_unmultiplied(
                            [nw as usize, nh as usize],
                            small.as_raw(),
                        )
                    });
                let _ = ptx.send((i, img));
                if i % 4 == 3 {
                    ctx3.request_repaint();
                }
            }
            ctx3.request_repaint();
        });
    }

    fn pump_library(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.scan_rx {
            if let Ok(games) = rx.try_recv() {
                self.games = games;
                self.scanning = false;
                self.scan_rx = None;
                self.start_game_details(ctx);
            }
        }
        if let Some(rx) = &self.poster_rx {
            let mut got = Vec::new();
            while let Ok(m) = rx.try_recv() {
                got.push(m);
            }
            for (i, img) in got {
                let tex = img.map(|im| {
                    ctx.load_texture(format!("poster{i}"), im, egui::TextureOptions::LINEAR)
                });
                self.posters.insert(i, tex);
            }
        }
        if let Some(rx) = &self.meta_rx {
            while let Ok((i, m)) = rx.try_recv() {
                self.meta.insert(i, m);
            }
        }
    }

    /// Add a game by hand and keep it: it comes back on the next start (#28).
    fn add_game(&mut self, path: PathBuf, ctx: &egui::Context) {
        library::remember_added(&path);
        let mut added = library::added_games(std::slice::from_ref(&path));
        if let Some(g) = added.pop() {
            self.games.insert(0, g);
            library::sort_and_dedupe(&mut self.games);
            self.posters.clear();
            self.meta.clear();
            self.start_game_details(ctx);
        }
        self.open_game(path);
    }

    /// Install / Update from a Games card: resolve Shipping exe, keep progress on the card.
    fn update_game(&mut self, path: PathBuf, index: usize) {
        // Prefer the library folder so collections (Mass Effect LE, …) keep every
        // sub-game candidate. Meta's single exe would wipe the list.
        let target = self
            .games
            .get(index)
            .map(|g| g.dir.clone())
            .filter(|d| d.is_dir())
            .or_else(|| self.meta.get(&index).map(|m| m.exe.clone()))
            .unwrap_or(path);
        self.exe_text = target.to_string_lossy().into_owned();
        self.refresh();
        if let Some(Ok(st)) = &self.status {
            if !st.problems.is_empty() {
                self.page = Page::Setup;
                return;
            }
            // Collections default to every sub-game selected; Install them all.
            self.engine = if st.opti {
                Engine::Opti
            } else {
                Engine::ReShade
            };
            self.updating = Some(index);
            self.start(None);
        } else {
            self.page = Page::Setup;
        }
    }

    /// Re-read one game after its install, so the card stops claiming an
    /// update is available the moment it is not.
    fn refresh_one(&mut self, index: usize, ctx: &egui::Context) {
        let Some(g) = self.games.get(index).cloned() else {
            return;
        };
        let (tx, rx) = channel();
        self.meta_one_rx = Some(rx);
        let ctx = ctx.clone();
        thread::spawn(move || {
            let latest = net::client()
                .map(|c| installer::Latest::fetch(&c))
                .unwrap_or_default();
            if let Some(m) = game::resolve_target(&g.dir)
                .and_then(|(exe, _)| game::inspect(&exe))
                .ok()
                .map(|st| meta_from_status(&st, &latest))
            {
                let _ = tx.send((index, m));
            }
            ctx.request_repaint();
        });
    }

    fn open_game(&mut self, path: PathBuf) {
        self.exe_text = path.to_string_lossy().into_owned();
        self.refresh();
        self.page = Page::Setup;
    }

    /// Re-check at most this often while the window is open.
    const UPDATE_CHECK_EVERY: std::time::Duration = std::time::Duration::from_secs(30 * 60);

    /// Called every frame: cheap, and only starts a request when the interval
    /// has passed and nothing else is in flight.
    fn maybe_recheck_update(&mut self) {
        if !update::ENABLED {
            return;
        }
        if self.update_rx.is_some() || !matches!(self.update, UpdateState::Idle) {
            return;
        }
        if self.last_update_check.elapsed() >= Self::UPDATE_CHECK_EVERY {
            self.start_update_check();
        }
    }

    fn start_update_check(&mut self) {
        if !update::ENABLED {
            return;
        }
        self.last_update_check = std::time::Instant::now();
        let (tx, rx) = channel::<UpdateState>();
        self.update_rx = Some(rx);
        thread::spawn(move || {
            let st = match update::check() {
                Ok(Some(av)) => UpdateState::Available(av),
                _ => UpdateState::Idle,
            };
            let _ = tx.send(st);
        });
    }

    fn start_update_download(&mut self, av: update::Available) {
        let (tx, rx) = channel::<UpdateState>();
        self.update_rx = Some(rx);
        self.update = UpdateState::Downloading(0, "Starting".into());
        thread::spawn(move || {
            let p_tx = tx.clone();
            let res = update::download_and_swap(&av, &move |pct, msg| {
                let _ = p_tx.send(UpdateState::Downloading(pct, msg.to_owned()));
            });
            match res {
                Ok(exe) => {
                    let _ = tx.send(UpdateState::Restarting);
                    if update::relaunch(&exe).is_ok() {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        std::process::exit(0);
                    }
                }
                Err(e) => {
                    let _ = tx.send(UpdateState::Failed(format!("{e:#}")));
                }
            }
        });
    }

    fn run_diagnose(&mut self) {
        let Some(exe) = self.exe() else { return };
        self.log.clear();
        self.progress_msg.clear();
        match diagnose::run(&exe) {
            Ok(findings) => {
                for f in findings {
                    let line = match f.level {
                        diagnose::Level::Ok => LogLine::Ok(format!("ok: {}", text::tidy(&f.text))),
                        diagnose::Level::Warn => {
                            LogLine::Plain(format!("warn: {}", text::tidy(&f.text)))
                        }
                        diagnose::Level::Bad => {
                            LogLine::Fail(format!("FAIL: {}", text::tidy(&f.text)))
                        }
                    };
                    self.log.push(line);
                }
            }
            Err(e) => self.log.push(LogLine::Fail(format!("{e:#}"))),
        }
    }

    fn pump_update(&mut self) {
        let Some(rx) = &self.update_rx else { return };
        let mut last = None;
        while let Ok(m) = rx.try_recv() {
            last = Some(m);
        }
        if let Some(m) = last {
            let skip =
                matches!(&m, UpdateState::Available(av) if av.version == self.skipped_version);
            self.update = if skip { UpdateState::Idle } else { m };
        }
    }

    fn pump(&mut self) {
        let Some(rx) = &self.rx else { return };
        let mut finished = None;
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Progress(p, s) => {
                    self.progress = p;
                    self.progress_msg = s;
                }
                Msg::Log(l) => self.log.push(l),
                Msg::Finished(r) => finished = Some(r),
            }
        }
        if let Some(r) = finished {
            self.rx = None;
            self.running = false;
            if let Some(i) = self.updating.take() {
                self.pending_refresh = Some(i);
            }
            match r {
                Ok(msg) => {
                    self.progress = 100;
                    self.progress_msg = "Done.".into();
                    self.log.push(LogLine::Plain(msg));
                }
                Err(e) => {
                    self.progress_msg = "Failed.".into();
                    self.log.push(LogLine::Fail(e.clone()));
                    self.last_error = Some(e);
                }
            }
            self.refresh();
        }
    }
}

struct Tile {
    title: &'static str,
    detail: &'static str,
    ok: fn(&GameStatus) -> bool,
    optional: bool,
}

const TILES_FEEDER: [Tile; 6] = [
    Tile {
        title: "ReShade",
        detail: "add-on build · dxgi.dll",
        ok: |s| s.reshade,
        optional: false,
    },
    Tile {
        title: "Shader headers",
        detail: "ReShade.fxh · ReShadeUI.fxh · DrawText.fxh",
        ok: |s| s.headers,
        optional: false,
    },
    Tile {
        title: "DLSS5-Feeder",
        detail: "dlss5-feed.addon64 · DLSS5_Feed.fx",
        ok: |s| s.feeder,
        optional: false,
    },
    Tile {
        title: "LumeniteFX",
        detail: "motion vectors · Kernel 2.0",
        ok: |s| s.lumenite,
        optional: false,
    },
    Tile {
        title: "DLSS 5 add-on · leaked",
        detail: "renodx-dlss5.addon64 · nvngx_dlssnr.dll",
        ok: |s| s.dlss5_addon && s.dlssnr,
        optional: false,
    },
    Tile {
        title: "nvngx_dlss.dll",
        detail: "DLSS runtime · the Feeder's NGX session needs it",
        ok: |s| s.dlss,
        optional: false,
    },
];

/// Same Feeder row as above, but the in-game half is addon32 on 32-bit titles.
const TILE_FEEDER32: Tile = Tile {
    title: "DLSS5-Feeder",
    detail: "dlss5-feed.addon32 · DLSS5_Feed.fx (detail residual + Optical Flow)",
    ok: |s| s.feeder,
    optional: false,
};

const TILE_OPTI: Tile = Tile {
    title: "OptiScaler + NR pass",
    detail: "Dagherbou fork as dxgi.dll · Insert opens its overlay",
    ok: |s| s.opti,
    optional: false,
};

/// The OptiScaler route's neural consumer is built into OptiScaler itself, so
/// it needs the model and nothing else. Showing the ReShade route's add-on row
/// here left a finished install reporting "missing" (#66).
const TILE_OPTI_MODEL: Tile = Tile {
    title: "DLSS 5 model \u{00b7} leaked",
    detail: "nvngx_dlssnr.dll \u{00b7} OptiScaler's own NR pass consumes it",
    ok: |s| s.dlssnr,
    optional: false,
};

const TILES_NATIVE: [Tile; 4] = [
    Tile {
        title: "Game DLSS",
        detail: "nvngx_dlss.dll shipped by the game · add-on hooks it directly",
        ok: |_| true,
        optional: false,
    },
    Tile {
        title: "ReShade",
        detail: "add-on build · dxgi.dll",
        ok: |s| s.reshade,
        optional: false,
    },
    Tile {
        title: "DLSS 5 add-on · leaked",
        detail: "renodx-dlss5.addon64 · nvngx_dlssnr.dll",
        ok: |s| s.dlss5_addon && s.dlssnr,
        optional: false,
    },
    Tile {
        title: "DX11 bridge",
        detail: "dlss5-bridge.addon64 (NIGos) · D3D11 games",
        ok: |s| s.bridge,
        optional: true,
    },
];

const TILE_UPSTREAM: Tile = Tile {
    title: "Neural Upstream \u{00b7} experimental",
    detail: "nvngx.dll.addon64 (matiasLombo) \u{00b7} nvngx_dlssnr.dll",
    ok: |s| s.upstream && s.dlssnr,
    optional: false,
};

const TILE_HOST: Tile = Tile {
    title: "host64 helper (32-bit game)",
    detail: "dlss5-feed-host64.exe + 64-bit ReShade · add-on and models live in host64\\",
    ok: |s| s.host_exe && s.host_reshade,
    optional: false,
};

const TILE_RENODX: Tile = Tile {
    title: "RenoDX HDR mod",
    detail: "game-specific renodx-*.addon64 · Home → Add-ons → RenoDX",
    ok: |s| s.renodx_mod.is_some(),
    optional: true,
};

const TILE_REFRAMEWORK: Tile = Tile {
    title: "REFramework",
    detail: "dinput8.dll · RE Engine games need it before ReShade",
    ok: |s| s.reframework,
    optional: false,
};

/// Shortens `text` until it measures within `max_w`, ending in an ellipsis.
///
/// egui will happily wrap a long title onto a second line that the caption has
/// no room for, and the clip rect then cuts that line through the middle of the
/// letters. One line that says it was shortened is honest; half a line is not.
fn truncate_to_fit(text: &str, max_w: f32, measure: impl Fn(&str) -> f32) -> String {
    if measure(text) <= max_w {
        return text.to_owned();
    }
    let mut end = text.len();
    while end > 0 {
        // Step back to a character boundary, never into the middle of one.
        end -= 1;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        let candidate = format!("{}\u{2026}", text[..end].trim_end());
        if measure(&candidate) <= max_w {
            return candidate;
        }
    }
    String::new()
}

fn tiles_for(
    st: Option<&GameStatus>,
    engine: Engine,
    renodx_on: bool,
    upstream_on: bool,
) -> Vec<&'static Tile> {
    let mut v = base_tiles(st, engine, upstream_on);
    if st.is_some_and(|s| s.re_engine) {
        v.insert(0, &TILE_REFRAMEWORK);
    }
    if st.is_some_and(|s| s.is32()) {
        v.insert(1, &TILE_HOST);
    }
    if renodx_on || st.is_some_and(|s| s.renodx_mod.is_some()) {
        v.push(&TILE_RENODX);
    }
    v
}

fn base_tiles(st: Option<&GameStatus>, engine: Engine, upstream_on: bool) -> Vec<&'static Tile> {
    match st.map(|s| s.mode) {
        Some(game::Mode::Native) if engine == Engine::Opti || st.is_some_and(|s| s.opti) => {
            vec![&TILES_NATIVE[0], &TILE_OPTI, &TILE_OPTI_MODEL]
        }
        Some(game::Mode::Native) => {
            let needs_bridge = st.is_some_and(|s| s.needs_bridge());
            let upstream = upstream_on || st.is_some_and(|s| s.upstream);
            TILES_NATIVE
                .iter()
                .filter(|t| t.title != "DX11 bridge" || needs_bridge)
                .map(|t| {
                    if upstream && t.title == "DLSS 5 add-on \u{00b7} leaked" {
                        &TILE_UPSTREAM
                    } else {
                        t
                    }
                })
                .collect()
        }
        _ => TILES_FEEDER
            .iter()
            .map(|t| {
                if t.title == "DLSS5-Feeder" && st.is_some_and(|s| s.is32()) {
                    &TILE_FEEDER32
                } else {
                    t
                }
            })
            .collect(),
    }
}

fn chip(ui: &mut egui::Ui, text: &str, color: Color32, outlined: bool) {
    Frame::new()
        .stroke(Stroke::new(1.0, if outlined { color } else { t::BORDER }))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(t::mono(10.0)).color(color));
        });
}

fn tile(ui: &mut egui::Ui, rect: egui::Rect, tl: &Tile, st: Option<&GameStatus>) {
    // A status row, not a control: no border, no fill, a check / ring / dash
    // glyph and two lines of text. The hover shows nothing clickable.
    let ok = st.map(tl.ok).unwrap_or(false);
    let dashed = tl.optional && !ok;
    let inner = rect.shrink2(Vec2::new(6.0, 4.0));
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            paint_status_glyph(ui, ok, tl.optional);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                let title_color = if ok {
                    t::TEXT
                } else if dashed {
                    t::TEXT_DIM
                } else {
                    t::TEXT_OFF
                };
                ui.label(
                    RichText::new(tl.title)
                        .font(t::plex_medium(12.5))
                        .color(title_color),
                );
                ui.label(
                    RichText::new(tl.detail)
                        .font(t::plex(10.5))
                        .color(if dashed { t::TEXT_DIM } else { t::TEXT_MUTED }),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (text, color) = if ok {
                    ("installed", t::SUCCESS)
                } else if dashed {
                    ("not needed", t::TEXT_DIM)
                } else {
                    ("missing", t::TEXT_MUTED)
                };
                ui.label(RichText::new(text).font(t::plex(10.5)).color(color));
            });
        },
    );
    // Hairline under each row, so the list reads as a table.
    ui.painter().hline(
        rect.left() + 6.0..=rect.right() - 6.0,
        rect.bottom(),
        Stroke::new(1.0, t::BORDER),
    );
}

/// Filled check (installed), hollow ring (missing), dash (optional, not needed).
fn paint_status_glyph(ui: &mut egui::Ui, ok: bool, optional: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    if ok {
        p.circle_filled(c, 7.0, t::SUCCESS);
        let s = Stroke::new(1.7, t::BG);
        p.line_segment([c + Vec2::new(-3.2, 0.2), c + Vec2::new(-1.1, 2.3)], s);
        p.line_segment([c + Vec2::new(-1.1, 2.3), c + Vec2::new(3.2, -2.1)], s);
    } else if optional {
        p.line_segment(
            [c + Vec2::new(-4.0, 0.0), c + Vec2::new(4.0, 0.0)],
            Stroke::new(1.6, t::RING_OFF),
        );
    } else {
        p.circle_stroke(c, 6.2, Stroke::new(1.4, t::TEXT_DIM));
    }
}

const CARD_GAP: f32 = 12.0;
const LIST_COVER_W: f32 = 92.0;
const LIST_CARD_H: f32 = 132.0;
const LIST_CARD_MIN_W: f32 = 420.0;

impl App {
    fn lang(&self) -> Language {
        self.settings.language
    }

    fn tr(&self, key: T) -> &'static str {
        self.lang().t(key)
    }

    fn games_page(&mut self, ui: &mut egui::Ui) {
        if self.store_icons.is_empty() {
            self.store_icons = load_store_icons(ui.ctx());
        }
        let installed_n = self.meta.values().filter(|m| m.installed).count();
        let total_n = self.games.len();
        let available_n = total_n.saturating_sub(installed_n);

        // Title + summary stats on one row (reference layout).
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(self.tr(T::YourGames))
                        .font(t::sora(22.0))
                        .color(t::TEXT),
                );
                ui.label(
                    RichText::new(self.tr(T::YourGamesSub))
                        .font(t::plex(13.0))
                        .color(t::TEXT_SECONDARY),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui_c::summary_stat_dot(
                    ui,
                    self.tr(T::AvailableCount),
                    available_n.to_string(),
                    t::WARNING,
                );
                ui_c::summary_stat_dot(
                    ui,
                    self.tr(T::InstalledCount),
                    installed_n.to_string(),
                    t::SUCCESS,
                );
                ui_c::summary_stat_dot(
                    ui,
                    self.tr(T::TotalGames),
                    total_n.to_string(),
                    t::PRIMARY,
                );
            });
        });
        ui.add_space(12.0);

        let search_hint = self.tr(T::SearchGames);
        let add_game_l = self.tr(T::AddGame);
        let add_folder_l = self.tr(T::AddFolder);
        let rescan_l = self.tr(T::Rescan);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let search_w = (ui.available_width() - 360.0).clamp(200.0, 420.0);
            let search = egui::TextEdit::singleline(&mut self.search)
                .font(t::plex(12.5))
                .hint_text(RichText::new(search_hint).color(t::TEXT_DIM))
                .desired_width(search_w);
            ui.add(search);
            if ui.add(ui_c::secondary_button(add_game_l)).clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Executables", &["exe", "bin"])
                    .pick_file()
                {
                    self.add_game(p, ui.ctx());
                }
            }
            if ui.add(ui_c::secondary_button(add_folder_l)).clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .set_title(self.tr(T::PickGameFolder))
                    .pick_folder()
                {
                    self.add_game(p, ui.ctx());
                }
            }
            if ui
                .add_enabled(
                    !self.scanning && !self.running,
                    ui_c::primary_button(rescan_l).min_size(Vec2::new(130.0, 40.0)),
                )
                .clicked()
            {
                self.start_scan(ui.ctx());
            }
        });
        ui.add_space(10.0);

        let needle = self.search.trim().to_ascii_lowercase();
        let mut clicked: Option<(PathBuf, usize)> = None;
        let mut forgotten: Option<PathBuf> = None;
        let mut update = false;
        let section_ours = self.tr(T::InstalledByTool);
        let section_manual = self.tr(T::AddedByYou);
        let empty_title = self.tr(T::NoGamesFound);
        let empty_hint = self.tr(T::NoGamesHint);
        egui::ScrollArea::vertical()
            .max_height(ui.available_height())
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                let avail = ui.available_width() - 14.0;
                let cols = ((avail + CARD_GAP) / (LIST_CARD_MIN_W + CARD_GAP))
                    .floor()
                    .max(1.0) as usize;
                let card_w =
                    ((avail - CARD_GAP * (cols as f32 - 1.0)) / cols as f32).max(LIST_CARD_MIN_W);
                let sections: [Option<Store>; 6] = [
                    None,
                    Some(Store::Manual),
                    Some(Store::Steam),
                    Some(Store::Epic),
                    Some(Store::Gog),
                    Some(Store::Xbox),
                ];
                for section in sections {
                    let idx: Vec<usize> = self
                        .games
                        .iter()
                        .enumerate()
                        .filter(|(i, g)| {
                            let ours = self.meta.get(i).is_some_and(|m| m.installed);
                            let in_section = match section {
                                None => ours,
                                Some(st) => g.store == st && !ours,
                            };
                            in_section
                                && (needle.is_empty()
                                    || g.title.to_ascii_lowercase().contains(&needle))
                        })
                        .map(|(i, _)| i)
                        .collect();
                    if idx.is_empty() {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(match section {
                                None => section_ours,
                                Some(Store::Manual) => section_manual,
                                Some(st) => st.label(),
                            })
                            .font(t::plex_semibold(13.5))
                            .color(t::TEXT),
                        );
                        ui_c::chip(ui, &idx.len().to_string(), ui_c::ChipTone::Neutral);
                    });
                    ui.add_space(6.0);
                    for row in idx.chunks(cols) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = CARD_GAP;
                            for &i in row {
                                let action = self.game_card_list(ui, card_w, i);
                                if action == CardAction::Forget {
                                    forgotten = Some(self.games[i].dir.clone());
                                }
                                if matches!(action, CardAction::Open | CardAction::Update) {
                                    update = action == CardAction::Update;
                                    let g = &self.games[i];
                                    let path = match game::resolve_target(&g.dir) {
                                        Ok(_) => g.dir.clone(),
                                        Err(_) => match &g.exe_hint {
                                            Some(e)
                                                if e.is_file() && game::exe_bitness(e).is_ok() =>
                                            {
                                                e.clone()
                                            }
                                            _ => g.dir.clone(),
                                        },
                                    };
                                    clicked = Some((path, i));
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }
                    ui.add_space(8.0);
                }
                if !self.scanning && self.games.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(empty_title)
                                .font(t::plex(13.0))
                                .color(t::TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new(empty_hint)
                                .font(t::plex(12.0))
                                .color(t::TEXT_DIM),
                        );
                    });
                }
            });
        if let Some(p) = forgotten {
            library::forget_added(&p);
            self.games
                .retain(|g| !(g.store == Store::Manual && g.dir == p));
        }
        if let Some((p, index)) = clicked {
            if update {
                self.update_game(p, index);
            } else {
                self.open_game(p);
            }
        }
    }

    /// Horizontal list card matching the redesign reference (cover | info | action).
    fn game_card_list(&self, ui: &mut egui::Ui, width: f32, i: usize) -> CardAction {
        let g = &self.games[i];
        let m = self.meta.get(&i);
        let installed = m.is_some_and(|m| m.installed);
        let stale = m.is_some_and(|m| !m.stale.is_empty());
        let mut action = CardAction::None;
        let path_full = g.dir.to_string_lossy().into_owned();
        let path_short = ui_c::truncate_path(&path_full, 42);
        let open_l = self.tr(T::OpenSetup);
        let install_l = if !installed {
            self.tr(T::Install)
        } else if stale {
            self.tr(T::Update)
        } else {
            self.tr(T::Reinstall)
        };

        let frame = Frame::new()
            .fill(t::SURFACE)
            .stroke(Stroke::new(1.0, t::BORDER))
            .corner_radius(t::card_rounding())
            .inner_margin(Margin::same(10));
        let inner = frame.show(ui, |ui| {
            ui.set_width((width - 20.0).max(200.0));
            ui.set_min_height(LIST_CARD_H - 20.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                let cover_size = Vec2::new(LIST_COVER_W, LIST_CARD_H - 24.0);
                let (_, cover_resp) = ui.allocate_exact_size(cover_size, egui::Sense::click());
                let cover = cover_resp.rect;
                let p = ui.painter();
                let cr = CornerRadius::same(8);
                match self.posters.get(&i) {
                    Some(Some(tex)) => {
                        egui::Image::from_texture(tex)
                            .fit_to_exact_size(cover.size())
                            .corner_radius(cr)
                            .paint_at(ui, cover);
                    }
                    _ => {
                        p.rect_filled(cover, cr, t::BG);
                        p.text(
                            cover.center(),
                            egui::Align2::CENTER_CENTER,
                            "…",
                            t::plex(12.0),
                            t::TEXT_MUTED,
                        );
                    }
                }
                if let Some(m) = m {
                    let api =
                        p.layout_no_wrap(m.api.to_owned(), t::plex_semibold(9.5), t::PRIMARY_HOVER);
                    let pad = Vec2::new(6.0, 3.0);
                    let chip = egui::Rect::from_min_size(
                        egui::pos2(cover.left() + 6.0, cover.top() + 6.0),
                        api.size() + pad * 2.0,
                    );
                    p.rect_filled(chip, t::chip_rounding(), t::PRIMARY_SOFT);
                    p.rect_stroke(
                        chip,
                        t::chip_rounding(),
                        Stroke::new(1.0, t::BORDER_ACTIVE),
                        StrokeKind::Inside,
                    );
                    p.galley(chip.min + pad, api, t::PRIMARY_HOVER);
                }
                if self.updating == Some(i) {
                    p.rect_filled(cover, cr, Color32::from_black_alpha(180));
                    let pct = self.progress.min(100);
                    p.text(
                        cover.center(),
                        egui::Align2::CENTER_CENTER,
                        fmt_n(self.tr(T::UpdatingInProgress), pct as usize),
                        t::plex_semibold(12.0),
                        t::TEXT,
                    );
                }

                ui.vertical(|ui| {
                    ui.set_min_width((width - LIST_COVER_W - 40.0).max(200.0));
                    ui.horizontal(|ui| {
                        let mr = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover()).1;
                        store_mark(ui, &self.store_icons, mr.rect, g.store, t::TEXT_OFF);
                        ui.label(
                            RichText::new(&g.title)
                                .font(t::plex_semibold(14.0))
                                .color(t::TEXT),
                        );
                    });
                    ui.label(
                        RichText::new(&path_short)
                            .font(t::mono(11.0))
                            .color(t::TEXT_MUTED),
                    )
                    .on_hover_text(&path_full);
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if let Some(m) = m {
                            if !m.stale.is_empty() {
                                ui_c::chip(ui, self.tr(T::StatusUpdate), ui_c::ChipTone::Warning);
                            } else if m.installed {
                                ui_c::chip(
                                    ui,
                                    self.tr(T::StatusInstalled),
                                    ui_c::ChipTone::Success,
                                );
                            } else {
                                ui_c::chip(
                                    ui,
                                    self.tr(T::StatusNotInstalled),
                                    ui_c::ChipTone::Warning,
                                );
                            }
                            if m.addon || m.installed {
                                ui_c::chip(ui, m.engine_path, ui_c::ChipTone::Neutral);
                            }
                            if m.ready && m.stale.is_empty() {
                                ui_c::chip(ui, "DLSS 5", ui_c::ChipTone::Primary);
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let primary = if installed && !stale {
                            open_l
                        } else {
                            install_l
                        };
                        let primary_action = if installed && !stale {
                            CardAction::Open
                        } else {
                            CardAction::Update
                        };
                        if ui
                            .add_enabled(
                                self.updating != Some(i) && !self.running,
                                ui_c::primary_button(primary).min_size(Vec2::new(160.0, 34.0)),
                            )
                            .clicked()
                        {
                            action = primary_action;
                        }
                        if installed && !stale {
                            // already primary open
                        } else if installed {
                            if ui
                                .add(
                                    ui_c::secondary_button(open_l).min_size(Vec2::new(120.0, 34.0)),
                                )
                                .clicked()
                            {
                                action = CardAction::Open;
                            }
                        }
                    });
                });
            });
        });

        let resp = inner.response.interact(egui::Sense::click());
        if resp.hovered() {
            ui.painter().rect_stroke(
                inner.response.rect,
                t::card_rounding(),
                Stroke::new(1.0, t::BORDER_ACTIVE),
                StrokeKind::Inside,
            );
        }
        if resp.clicked() && action == CardAction::None && self.updating != Some(i) {
            action = CardAction::Open;
        }
        if installed || g.store == Store::Manual {
            resp.context_menu(|ui| {
                if installed {
                    let label = if stale {
                        self.tr(T::Update)
                    } else {
                        self.tr(T::Reinstall)
                    };
                    if ui
                        .add_enabled(!self.running, egui::Button::new(label))
                        .clicked()
                    {
                        action = CardAction::Update;
                        ui.close();
                    }
                }
                if g.store == Store::Manual && ui.button(self.tr(T::ForgetGame)).clicked() {
                    action = CardAction::Forget;
                    ui.close();
                }
            });
        }
        action
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui_c::page_title(ui, self.tr(T::Settings), Some(self.tr(T::General)));

        let lang_title = self.tr(T::Language);
        ui_c::section_card(ui, lang_title, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                for lang in [Language::Ru, Language::En] {
                    let on = self.settings.language == lang;
                    if ui
                        .selectable_label(
                            on,
                            RichText::new(lang.label())
                                .font(t::plex_medium(13.0))
                                .color(if on { t::TEXT } else { t::TEXT_SECONDARY }),
                        )
                        .clicked()
                        && !on
                    {
                        self.settings.language = lang;
                        let _ = self.settings.save();
                    }
                }
            });
        });
        ui.add_space(10.0);

        let quality_title = self.tr(T::QualityPreset);
        ui_c::section_card(ui, quality_title, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for c in [
                    QualityChoice::Auto,
                    QualityChoice::Low,
                    QualityChoice::Medium,
                    QualityChoice::High,
                ] {
                    let on = self.settings.quality_choice() == c;
                    let btn = egui::Button::new(
                        RichText::new(c.label())
                            .font(t::plex_medium(12.5))
                            .color(if on {
                                Color32::WHITE
                            } else {
                                t::TEXT_SECONDARY
                            }),
                    )
                    .fill(if on { t::PRIMARY } else { Color32::TRANSPARENT })
                    .stroke(Stroke::new(
                        1.0,
                        if on { t::PRIMARY } else { t::BORDER_STRONG },
                    ))
                    .corner_radius(t::control_rounding())
                    .min_size(Vec2::new(72.0, 32.0));
                    if ui.add(btn).clicked() {
                        self.settings.set_quality_choice(c);
                    }
                }
            });
        });
        ui.add_space(10.0);

        let feeder_title = self.tr(T::FeederDefaults);
        ui_c::section_card(ui, feeder_title, |ui| {
            ui.label(
                RichText::new(self.tr(T::FeederDefaultsHint))
                    .font(t::plex(11.0))
                    .color(t::TEXT_DIM),
            );
            ui.add_space(6.0);
            let k = &mut self.settings.knobs;
            let mut work = k.work_resolution.unwrap_or(100);
            if ui
                .add(egui::Slider::new(&mut work, 50..=100).text("work_resolution %"))
                .changed()
            {
                k.work_resolution = Some(work);
            }
            let mut ofa = k.ofa_enabled.unwrap_or(false);
            if ui.checkbox(&mut ofa, "ofa_enabled").changed() {
                k.ofa_enabled = Some(ofa);
            }
            let mut reset = k.reset_mode.unwrap_or(2);
            if ui
                .add(
                    egui::Slider::new(&mut reset, 0..=2)
                        .text("reset_mode (0 off / 1 every / 2 adaptive)"),
                )
                .changed()
            {
                k.reset_mode = Some(reset);
            }
            let mut ls = k.light_stab.unwrap_or(false);
            if ui.checkbox(&mut ls, "light_stab").changed() {
                k.light_stab = Some(ls);
            }
            let mut ev = k.engine_velocity.unwrap_or(true);
            if ui.checkbox(&mut ev, "engine_velocity").changed() {
                k.engine_velocity = Some(ev);
            }
        });
        ui.add_space(10.0);

        let overlay_title = self.tr(T::OverlayDiagnostics);
        ui_c::section_card(ui, overlay_title, |ui| {
            ui.add(
                egui::Slider::new(&mut self.settings.overlay.log_detail, 0..=2).text("log_detail"),
            );
            ui.add(
                egui::Slider::new(&mut self.settings.overlay.evaluate_stride, 1..=4)
                    .text("evaluate_stride"),
            );
            ui.add(
                egui::Slider::new(&mut self.settings.overlay.log_frames, 0..=20).text("log_frames"),
            );
        });
        ui.add_space(14.0);

        let save_l = self.tr(T::SaveSettings);
        let apply_l = self.tr(T::ApplyToGame);
        let reset_l = self.tr(T::ResetFeederDefaults);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            if ui.add(ui_c::primary_button(save_l)).clicked() {
                if let Err(e) = self.settings.save() {
                    self.last_error = Some(format!("{e:#}"));
                }
            }
            if ui
                .add_enabled(
                    self.resolved_exe.is_some(),
                    ui_c::secondary_button(apply_l),
                )
                .clicked()
            {
                let _ = self.settings.save();
                self.apply_settings_to_game();
            }
            if ui
                .add(ui_c::secondary_button(reset_l))
                .on_hover_text(
                    "Restore form to Feeder stock: work_resolution=100, ofa off (grid 2 / perf 10), \
                     reset_mode=2 adaptive, light_stab off, engine_velocity on, overlay log_detail=1 / \
                     evaluate_stride=1 / log_frames=3, quality Auto.",
                )
                .clicked()
            {
                self.settings.reset_to_feeder_defaults();
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Feeder built-in defaults = stock add-on cfg (not your settings.json). \
                 Reset above copies those into this form; Save to persist.",
            )
            .font(t::plex(11.0))
            .color(t::TEXT_DIM),
        );
        ui.label(
            RichText::new(format!(
                "{}: {}",
                self.tr(T::Path),
                Settings::path().display()
            ))
            .font(t::mono(11.0))
            .color(t::TEXT_DIM),
        );
    }

    fn knobs_panel(&mut self, ui: &mut egui::Ui) {
        let title = self.tr(T::FeederKnobs);
        let basics_l = self.tr(T::Basics);
        let motion_l = self.tr(T::MotionVectors);
        let advanced_l = self.tr(T::Advanced);
        let write_l = self.tr(T::WriteCfg);
        let reload_l = self.tr(T::ReloadFromDisk);
        ui_c::card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(title)
                    .font(t::plex_semibold(13.5))
                    .color(t::TEXT),
            );
            ui.add_space(8.0);
            if self.knobs.is_none() {
                if let Some(err) = &self.knobs_err {
                    ui.label(
                        RichText::new(err.clone())
                            .font(t::plex(12.0))
                            .color(t::TEXT_MUTED),
                    );
                    if ui
                        .add(ui_c::primary_button(self.tr(T::Install)))
                        .clicked()
                    {
                        self.start(None);
                    }
                } else {
                    ui.label(
                        RichText::new(self.tr(T::SelectGameFirst))
                            .font(t::plex(12.0))
                            .color(t::TEXT_DIM),
                    );
                }
                return;
            }

            let mut changed = false;
            {
                let k = self.knobs.as_mut().unwrap();
                ui.label(
                    RichText::new(basics_l)
                        .font(t::plex_medium(12.0))
                        .color(t::TEXT_SOFT),
                );
                changed |= ui
                    .add(
                        egui::Slider::new(&mut k.work_resolution, 50..=100)
                            .text("work_resolution %"),
                    )
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut k.work_sharpness, 0.0..=1.0).text("work_sharpness"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut k.reset_mode, 0..=2).text("reset_mode"))
                    .changed();
                changed |= ui.checkbox(&mut k.light_stab, "light_stab").changed();
                changed |= ui
                    .checkbox(&mut k.engine_velocity, "engine_velocity")
                    .changed();

                ui.add_space(8.0);
                ui.label(
                    RichText::new(motion_l)
                        .font(t::plex_medium(12.0))
                        .color(t::TEXT_SOFT),
                );
                changed |= ui.checkbox(&mut k.ofa_enabled, "Optical Flow (ofa)").changed();
                if k.ofa_enabled {
                    changed |= ui
                        .add(egui::Slider::new(&mut k.ofa_grid, 0..=4).text("ofa_grid"))
                        .changed();
                }

                ui.add_space(4.0);
                egui::CollapsingHeader::new(
                    RichText::new(advanced_l)
                        .font(t::plex_medium(12.0))
                        .color(t::TEXT_SOFT),
                )
                .default_open(false)
                .show(ui, |ui| {
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut k.evaluate_stride, 1..=4)
                                .text("evaluate_stride"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut k.appearance_threshold, 0.01..=0.12)
                                .text("appearance_threshold"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut k.lighting_threshold, 0.01..=0.12)
                                .text("lighting_threshold"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut k.detail_threshold, 0.005..=0.08)
                                .text("detail_threshold"),
                        )
                        .changed();
                });
            }
            if changed {
                self.knobs_dirty = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let write_text = if self.knobs_dirty {
                    format!("{write_l} *")
                } else {
                    write_l.to_owned()
                };
                if ui
                    .add_enabled(self.knobs.is_some(), ui_c::primary_button(write_text))
                    .clicked()
                {
                    if let (Some(exe), Some(k)) = (self.exe(), self.knobs.clone()) {
                        if let Some(dir) = exe.parent() {
                            match feeder_cfg::save(dir, &k) {
                                Ok(()) => {
                                    self.knobs_dirty = false;
                                    self.log.push(LogLine::Ok(
                                        "Wrote dlss5-feed.cfg + FX uniforms (Feeder reloads ~60 frames / next launch)"
                                            .into(),
                                    ));
                                }
                                Err(e) => self.log.push(LogLine::Fail(format!("{e:#}"))),
                            }
                        }
                    }
                }
                if ui.add(ui_c::secondary_button(reload_l)).clicked() {
                    self.reload_knobs_and_perf();
                }
            });
        });
    }

    fn hotkeys_panel(&mut self, ui: &mut egui::Ui) {
        let title = self.tr(T::Hotkeys);
        let hint = self.tr(T::HotkeysHint);
        let write_l = self.tr(T::WriteHotkeys);
        let reload_l = self.tr(T::ReloadFromDisk);
        let press_l = self.tr(T::PressKey);
        let overlay_rs = self.tr(T::OverlayReshade);
        let toggle_nr = self.tr(T::ToggleNr);
        let nr_shot = self.tr(T::NrScreenshot);
        let overlay_opti = self.tr(T::OverlayOpti);
        let reset_l = self.tr(T::Reset);
        let clear_l = self.tr(T::Clear);

        ui_c::card_elevated().show(ui, |ui| {
            ui.label(
                RichText::new(title)
                    .font(t::plex_semibold(13.5))
                    .color(t::TEXT),
            );
            ui.label(
                RichText::new(hint)
                    .font(t::plex(11.0))
                    .color(t::TEXT_DIM),
            );
            ui.add_space(8.0);

            if self.hotkeys.is_none() {
                if let Some(err) = &self.hotkeys_err {
                    ui.label(
                        RichText::new(err.clone())
                            .font(t::plex(12.0))
                            .color(t::TEXT_MUTED),
                    );
                } else {
                    ui.label(
                        RichText::new(self.tr(T::SelectGameFirst))
                            .font(t::plex(12.0))
                            .color(t::TEXT_DIM),
                    );
                }
                return;
            }

            // Capture a key while a row is armed.
            if let Some(slot) = self.hotkey_capture {
                let mut caught: Option<(u8, bool, bool, bool)> = None;
                let mut cancel = false;
                ui.input(|i| {
                    for ev in &i.events {
                        if let egui::Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } = ev
                        {
                            if *key == egui::Key::Escape {
                                cancel = true;
                                break;
                            }
                            if let Some(vk) = hotkeys::egui_to_vk(*key) {
                                caught = Some((
                                    vk,
                                    modifiers.ctrl || modifiers.command,
                                    modifiers.shift,
                                    modifiers.alt,
                                ));
                                break;
                            }
                        }
                    }
                });
                if cancel {
                    self.hotkey_capture = None;
                } else if let Some((vk, ctrl, shift, alt)) = caught {
                    if let Some(h) = self.hotkeys.as_mut() {
                        match slot {
                            HotkeySlot::ReshadeOverlay => {
                                h.reshade_overlay = KeyChord {
                                    vk,
                                    ctrl,
                                    shift,
                                    alt,
                                };
                            }
                            HotkeySlot::NrToggle => h.nr_toggle = vk,
                            HotkeySlot::NrScreenshot => h.nr_screenshot = vk,
                            HotkeySlot::OptiMenu => h.opti_menu = vk,
                        }
                        self.hotkeys_dirty = true;
                    }
                    self.hotkey_capture = None;
                }
            }

            let capturing = self.hotkey_capture;
            let show_reshade = self.hotkeys.as_ref().is_some_and(|h| h.has_reshade);
            let show_opti = self.hotkeys.as_ref().is_some_and(|h| h.has_opti);

            if show_reshade {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("ReShade")
                        .font(t::plex_medium(12.0))
                        .color(t::TEXT_SOFT),
                );
                let host64 = self
                    .hotkeys
                    .as_ref()
                    .is_some_and(|h| h.consumer_dir != h.game_dir);
                if host64 {
                    ui.label(
                        RichText::new(
                            "32-bit game: NR toggle/screenshot keys are written to host64\\ReShade.ini \
                             (the add-on runs there). Prefer uncommon keys — the helper sees them globally.",
                        )
                        .font(t::plex(11.0))
                        .color(t::WARN),
                    );
                }
                self.hotkey_row(
                    ui,
                    HotkeySlot::ReshadeOverlay,
                    overlay_rs,
                    capturing,
                    reset_l,
                    clear_l,
                    |h| h.reshade_overlay.label(),
                    |h| h.reshade_overlay = KeyChord::new(hotkeys::VK_HOME),
                    |h| h.reshade_overlay = KeyChord::unbound(),
                );
                self.hotkey_row(
                    ui,
                    HotkeySlot::NrToggle,
                    toggle_nr,
                    capturing,
                    reset_l,
                    clear_l,
                    |h| hotkeys::vk_name(h.nr_toggle),
                    |h| h.nr_toggle = hotkeys::VK_F6,
                    |h| h.nr_toggle = 0,
                );
                self.hotkey_row(
                    ui,
                    HotkeySlot::NrScreenshot,
                    nr_shot,
                    capturing,
                    reset_l,
                    clear_l,
                    |h| hotkeys::vk_name(h.nr_screenshot),
                    |h| h.nr_screenshot = hotkeys::VK_F5,
                    |h| h.nr_screenshot = 0,
                );
            }

            if show_opti {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("OptiScaler")
                        .font(t::plex_medium(12.0))
                        .color(t::TEXT_SOFT),
                );
                self.hotkey_row(
                    ui,
                    HotkeySlot::OptiMenu,
                    overlay_opti,
                    capturing,
                    reset_l,
                    clear_l,
                    |h| hotkeys::vk_name(h.opti_menu),
                    |h| h.opti_menu = hotkeys::VK_INSERT,
                    |h| h.opti_menu = 0,
                );
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let write_text = if self.hotkeys_dirty {
                    format!("{write_l} *")
                } else {
                    write_l.to_owned()
                };
                if ui
                    .add_enabled(self.hotkeys.is_some(), ui_c::primary_button(write_text))
                    .clicked()
                {
                    if let Some(h) = self.hotkeys.clone() {
                        let targets = self.install_targets();
                        let root = self.input_path();
                        let results = if targets.len() > 1 {
                            hotkeys::save_for_exes(&h, &targets)
                        } else if let Some(exe) = targets.first() {
                            vec![(exe.clone(), hotkeys::save_for_exe(&h, exe))]
                        } else {
                            vec![(PathBuf::new(), hotkeys::save(&h))]
                        };
                        let mut ok_n = 0usize;
                        let mut fail_n = 0usize;
                        for (exe, res) in results {
                            let label = if exe.as_os_str().is_empty() {
                                "game".into()
                            } else {
                                game::exe_label(&root, &exe)
                            };
                            match res {
                                Ok(()) => {
                                    ok_n += 1;
                                    self.log.push(LogLine::Ok(format!(
                                        "[{label}] wrote hotkeys (restart that game to apply)"
                                    )));
                                }
                                Err(e) => {
                                    fail_n += 1;
                                    self.log
                                        .push(LogLine::Fail(format!("[{label}] hotkeys: {e:#}")));
                                }
                            }
                        }
                        if fail_n == 0 {
                            self.hotkeys_dirty = false;
                        }
                        if ok_n > 1 {
                            self.log.push(LogLine::Ok(format!(
                                "Hotkeys written to {ok_n} games in this collection"
                            )));
                        }
                    }
                }
                if ui.add(ui_c::secondary_button(reload_l)).clicked() {
                    self.reload_hotkeys();
                }
                if capturing.is_some() {
                    ui.label(
                        RichText::new(press_l)
                            .font(t::plex(12.0))
                            .color(t::WARN),
                    );
                }
            });
        });
    }

    fn hotkey_row(
        &mut self,
        ui: &mut egui::Ui,
        slot: HotkeySlot,
        label: &str,
        capturing: Option<HotkeySlot>,
        reset_l: &str,
        clear_l: &str,
        current: impl Fn(&GameHotkeys) -> String,
        reset: impl Fn(&mut GameHotkeys),
        clear: impl Fn(&mut GameHotkeys),
    ) {
        let armed = capturing == Some(slot);
        let value = self
            .hotkeys
            .as_ref()
            .map(current)
            .unwrap_or_else(|| "—".into());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(label)
                    .font(t::plex(12.0))
                    .color(t::TEXT_MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new(clear_l).font(t::plex(11.0)))
                            .min_size(Vec2::new(52.0, 22.0)),
                    )
                    .clicked()
                {
                    if let Some(h) = self.hotkeys.as_mut() {
                        clear(h);
                        self.hotkeys_dirty = true;
                    }
                    if self.hotkey_capture == Some(slot) {
                        self.hotkey_capture = None;
                    }
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new(reset_l).font(t::plex(11.0)))
                            .min_size(Vec2::new(52.0, 22.0)),
                    )
                    .clicked()
                {
                    if let Some(h) = self.hotkeys.as_mut() {
                        reset(h);
                        self.hotkeys_dirty = true;
                    }
                    if self.hotkey_capture == Some(slot) {
                        self.hotkey_capture = None;
                    }
                }
                let btn_text = if armed {
                    "…".to_owned()
                } else {
                    value.clone()
                };
                let btn = egui::Button::new(
                    RichText::new(btn_text).font(t::mono(12.0)).color(if armed {
                        t::WARN
                    } else {
                        t::TEXT
                    }),
                )
                .min_size(Vec2::new(120.0, 22.0));
                if ui.add(btn).clicked() {
                    self.hotkey_capture = if armed { None } else { Some(slot) };
                }
            });
        });
    }
}

/// The stores' own marks (Simple Icons, CC0 1.0), white PNGs tinted at paint time.
const STORE_ICON_PNG: [(Store, &[u8]); 4] = [
    (Store::Steam, include_bytes!("../assets/store-steam.png")),
    (Store::Xbox, include_bytes!("../assets/store-xbox.png")),
    (Store::Epic, include_bytes!("../assets/store-epic.png")),
    (Store::Gog, include_bytes!("../assets/store-gog.png")),
];

fn load_store_icons(ctx: &egui::Context) -> HashMap<Store, egui::TextureHandle> {
    STORE_ICON_PNG
        .iter()
        .filter_map(|(store, png)| {
            let img = image::load_from_memory(png).ok()?.to_rgba8();
            let (w, h) = img.dimensions();
            let ci =
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
            Some((
                *store,
                ctx.load_texture(
                    format!("store-{}", store.label()),
                    ci,
                    egui::TextureOptions::LINEAR,
                ),
            ))
        })
        .collect()
}

fn store_mark(
    ui: &egui::Ui,
    icons: &HashMap<Store, egui::TextureHandle>,
    r: egui::Rect,
    store: Store,
    color: Color32,
) {
    if let Some(tex) = icons.get(&store) {
        egui::Image::from_texture(tex)
            .fit_to_exact_size(r.size())
            .tint(color)
            .paint_at(ui, r);
    }
}

fn about_page(ui: &mut egui::Ui, lang: Language) {
    let tr = |k: T| lang.t(k);
    ui_c::page_title(ui, tr(T::AboutTitle), None);
    ui.label(
        RichText::new(concat!("DLSS5oneclick v", env!("CARGO_PKG_VERSION")))
            .font(t::plex_semibold(14.0))
            .color(t::TEXT),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(tr(T::CommunityForkOf))
                .font(t::plex(12.5))
                .color(t::TEXT_SECONDARY),
        );
        ui.hyperlink_to(
            RichText::new("faisalkindi/DLSS5oneclick")
                .font(t::plex(12.5))
                .color(t::PRIMARY_HOVER),
            "https://github.com/faisalkindi/DLSS5oneclick",
        );
    });
    ui.add_space(12.0);

    ui_c::section_card(ui, tr(T::ForkChanges), |ui| {
        for key in [
            T::ForkHotkeys,
            T::ForkCollections,
            T::ForkHotkeyProp,
            T::ForkNoUpdate,
            T::ForkI18n,
        ] {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("•")
                        .font(t::plex(12.5))
                        .color(t::PRIMARY_HOVER),
                );
                ui.label(
                    RichText::new(tr(key))
                        .font(t::plex(12.5))
                        .color(t::TEXT_SOFT),
                );
            });
        }
    });
    ui.add_space(10.0);

    ui_c::section_card(ui, tr(T::Credits), |ui| {
        ui.label(
            RichText::new(tr(T::CreditsBlurb))
                .font(t::plex(12.0))
                .color(t::TEXT_MUTED),
        );
        ui.add_space(4.0);
        for (name, url) in [
            ("crosire — ReShade", "https://reshade.me"),
            (
                "clshortfuse and the RenoDX community",
                "https://github.com/clshortfuse/renodx",
            ),
            (
                "jlrouzies-fr — DLSS5-Feeder",
                "https://github.com/jlrouzies-fr/DLSS5-Feeder",
            ),
            (
                "umar-afzaal — LumeniteFX",
                "https://github.com/umar-afzaal/LumeniteFX",
            ),
            (
                "RankFTW — RHI and rhi-repo",
                "https://github.com/RankFTW/RHI",
            ),
            (
                "NIGos — dlss5-bridge",
                "https://github.com/NIGos/dlss5-bridge",
            ),
            (
                "Dagherbou — OptiScaler_DLSSNR",
                "https://github.com/Dagherbou/OptiScaler_DLSSNR",
            ),
            (
                "praydog — REFramework",
                "https://github.com/praydog/REFramework",
            ),
            (
                "Source, issues and releases",
                "https://github.com/branch-danya-dev/DLSS5oneclick",
            ),
        ] {
            ui.hyperlink_to(
                RichText::new(name).font(t::plex(12.0)).color(t::TEXT_OFF),
                url,
            );
        }
    });
    ui.add_space(10.0);

    ui_c::section_card(ui, tr(T::BuildInfo), |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{}:", tr(T::Version)))
                    .font(t::plex(12.0))
                    .color(t::TEXT_MUTED),
            );
            ui.label(
                RichText::new(env!("CARGO_PKG_VERSION"))
                    .font(t::mono(12.0))
                    .color(t::TEXT),
            );
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{}:", tr(T::Architecture)))
                    .font(t::plex(12.0))
                    .color(t::TEXT_MUTED),
            );
            ui.label(RichText::new(tr(T::X64)).font(t::mono(12.0)).color(t::TEXT));
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{}:", tr(T::UpdateStatus)))
                    .font(t::plex(12.0))
                    .color(t::TEXT_MUTED),
            );
            ui.label(
                RichText::new(tr(T::SelfUpdateDisabled))
                    .font(t::plex(12.0))
                    .color(t::TEXT_SOFT),
            );
        });
    });
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("exe", self.exe_text.clone());
        storage.set_string("skip_version", self.skipped_version.clone());
        storage.set_string(
            "tip_dismissed",
            if self.tip_dismissed { "1" } else { "0" }.into(),
        );
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump();
        self.pump_renodx();
        if let Some(i) = self.pending_refresh.take() {
            self.refresh_one(i, ui.ctx());
        }
        if let Some(rx) = &self.meta_one_rx {
            if let Ok((i, m)) = rx.try_recv() {
                self.meta.insert(i, m);
                self.meta_one_rx = None;
            }
        }
        self.maybe_recheck_update();
        if self.running || matches!(self.renodx, RenodxLookup::Pending) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }

        let (ok_status, mut problems, complete) = match &self.status {
            Some(Ok(s)) => (Some(s.clone()), s.problems.clone(), s.complete()),
            Some(Err(e)) => (None, vec![e.clone()], false),
            None => (None, vec![], false),
        };

        // A Vulkan game blocks the ReShade engine only; OptiScaler reaches it (#46).
        if self.engine == Engine::ReShade {
            if let Some(p) = ok_status
                .as_ref()
                .and_then(game::GameStatus::reshade_engine_problem)
            {
                problems.push(p);
            }
        }
        self.pump_library(ui.ctx());
        if self.scanning || self.poster_rx.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(120));
        }

        // ── top ribbon: logo · nav · status · lang · support ─────
        egui::Panel::top("ribbon")
            .resizable(false)
            .show_separator_line(true)
            .frame(
                Frame::new()
                    .fill(t::HEADER)
                    .inner_margin(Margin {
                        left: 20,
                        right: 20,
                        top: 10,
                        bottom: 10,
                    })
                    .stroke(Stroke::new(1.0, t::BORDER)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
                    logo::paint_mark(ui.painter(), rect, t::PRIMARY, t::BG);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            RichText::new("DLSS5oneclick")
                                .font(t::sora(15.0))
                                .color(t::TEXT),
                        );
                        ui.label(
                            RichText::new(self.tr(T::Tagline))
                                .font(t::plex(10.5))
                                .color(t::TEXT_MUTED),
                        );
                    });
                    ui.add_space(18.0);
                    let setup_enabled = self.resolved_exe.is_some();
                    let tabs = [
                        (Page::Games, T::Games, true),
                        (Page::Setup, T::Setup, setup_enabled),
                        (Page::Settings, T::Settings, true),
                        (Page::About, T::About, true),
                    ];
                    for (page, key, enabled) in tabs {
                        let active = self.page == page;
                        if ui_c::nav_tab(ui, self.tr(key), active, enabled).clicked() {
                            self.page = page;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        // Ko-fi — secondary, rightmost
                        if ui
                            .add(
                                ui_c::secondary_button(self.tr(T::Support))
                                    .min_size(Vec2::new(96.0, 34.0)),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(KOFI_URL));
                        }
                        // RU | EN language toggle
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            let ru = self.settings.language == Language::Ru;
                            let en = self.settings.language == Language::En;
                            let lang_btn = |label: &str, on: bool| {
                                egui::Button::new(
                                    RichText::new(label)
                                        .font(t::plex_medium(12.0))
                                        .color(if on { t::TEXT } else { t::TEXT_MUTED }),
                                )
                                .fill(if on {
                                    t::PRIMARY_SOFT
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(Stroke::new(
                                    1.0,
                                    if on { t::BORDER_ACTIVE } else { t::BORDER },
                                ))
                                .corner_radius(t::control_rounding())
                                .min_size(Vec2::new(36.0, 28.0))
                            };
                            if ui.add(lang_btn("RU", ru)).clicked() && !ru {
                                self.settings.language = Language::Ru;
                                let _ = self.settings.save();
                            }
                            ui.label(RichText::new("|").font(t::plex(11.0)).color(t::TEXT_DIM));
                            if ui.add(lang_btn("EN", en)).clicked() && !en {
                                self.settings.language = Language::En;
                                let _ = self.settings.save();
                            }
                        });
                        if ui.available_width() > 72.0 {
                            ui.label(
                                RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                                    .font(t::plex(11.5))
                                    .color(t::TEXT_DIM),
                            );
                        }
                        if ui.available_width() > 110.0 {
                            ui_c::chip(ui, self.tr(T::LeakedBuild), ui_c::ChipTone::Primary);
                        }
                        if ui.available_width() > 72.0 {
                            Frame::new()
                                .fill(t::SUCCESS_SOFT)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(0x2e, 0x6b, 0x45)))
                                .corner_radius(t::chip_rounding())
                                .inner_margin(Margin::symmetric(8, 4))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 6.0;
                                        let (r, _) = ui.allocate_exact_size(
                                            Vec2::splat(8.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(r.center(), 3.5, t::SUCCESS);
                                        ui.label(
                                            RichText::new(self.tr(T::Ready))
                                                .font(t::plex_medium(12.0))
                                                .color(t::SUCCESS),
                                        );
                                    });
                                });
                        }
                    });
                });
            });

        self.pump_update();
        match self.update.clone() {
            UpdateState::Idle => {}
            UpdateState::Available(av) => {
                egui::Panel::top("update_bar")
                    .frame(
                        Frame::new()
                            .fill(t::TILE)
                            .inner_margin(Margin {
                                left: 18,
                                right: 28,
                                top: 8,
                                bottom: 8,
                            })
                            .stroke(Stroke::new(1.0, t::BORDER)),
                    )
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.label(
                                RichText::new(fmt_vc(
                                    self.tr(T::VersionAvailable),
                                    &av.version,
                                    update::CURRENT,
                                ))
                                .font(t::plex_medium(12.5))
                                .color(t::TEXT),
                            );
                            let upd = egui::Button::new(
                                RichText::new(self.tr(T::Update))
                                    .font(t::plex_semibold(12.5))
                                    .color(t::BG),
                            )
                            .fill(t::ACCENT)
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(6));
                            if ui.add(upd).clicked() {
                                self.start_update_download(av.clone());
                            }
                            if ui.button(self.tr(T::UpdateLater)).clicked() {
                                self.update = UpdateState::Idle;
                            }
                            if ui.button(self.tr(T::SkipVersion)).clicked() {
                                self.skipped_version = av.version.clone();
                                self.update = UpdateState::Idle;
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.hyperlink_to(
                                    RichText::new(self.tr(T::ReleaseNotes)).font(t::plex(11.5)),
                                    format!(
                                        "https://github.com/{}/releases/tag/{}",
                                        update::REPO,
                                        av.tag
                                    ),
                                );
                            });
                        });
                    });
            }
            UpdateState::Downloading(pct, msg) => {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
                egui::Panel::top("update_bar")
                    .frame(
                        Frame::new()
                            .fill(t::TILE)
                            .inner_margin(Margin {
                                left: 18,
                                right: 28,
                                top: 8,
                                bottom: 8,
                            })
                            .stroke(Stroke::new(1.0, t::BORDER)),
                    )
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} {msg}",
                                fmt_n(self.tr(T::UpdatingPct), pct as usize)
                            ))
                            .font(t::plex(12.0))
                            .color(t::TEXT_MUTED),
                        );
                    });
            }
            UpdateState::Restarting => {
                egui::Panel::top("update_bar")
                    .frame(
                        Frame::new()
                            .fill(t::TILE)
                            .inner_margin(Margin {
                                left: 18,
                                right: 28,
                                top: 8,
                                bottom: 8,
                            })
                            .stroke(Stroke::new(1.0, t::BORDER)),
                    )
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(self.tr(T::UpdatedRestarting))
                                .font(t::plex(12.0))
                                .color(t::ACCENT),
                        );
                    });
            }
            UpdateState::Failed(e) => {
                egui::Panel::top("update_bar")
                    .frame(
                        Frame::new()
                            .fill(t::TILE)
                            .inner_margin(Margin {
                                left: 18,
                                right: 28,
                                top: 8,
                                bottom: 8,
                            })
                            .stroke(Stroke::new(1.0, t::BORDER)),
                    )
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{}: {e}", self.tr(T::UpdateFailed)))
                                    .font(t::plex(12.0))
                                    .color(t::DANGER),
                            );
                            if ui.button(self.tr(T::Dismiss)).clicked() {
                                self.update = UpdateState::Idle;
                            }
                        });
                    });
            }
        }

        egui::CentralPanel::default()
            .frame(Frame::new().fill(t::BG).inner_margin(Margin {
                left: 24,
                right: 28,
                top: 16,
                bottom: 14,
            }))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                ui_c::content_column(ui, |ui| {
                if !self.tip_dismissed {
                    if ui_c::info_banner(ui, self.tr(T::TipTitle), self.tr(T::TipBody)) {
                        self.tip_dismissed = true;
                    }
                    ui.add_space(10.0);
                }
                match self.page {
                    Page::Games => {
                        self.games_page(ui);
                        return;
                    }
                    Page::Settings => {
                        // Same fix Setup got: on a 768 px screen the Save /
                        // Apply / Reset row is below the edge, and there was no
                        // way to reach it (#64).
                        egui::ScrollArea::both()
                            .max_height(ui.available_height())
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.settings_page(ui));
                        return;
                    }
                    Page::About => {
                        egui::ScrollArea::vertical()
                            .max_height(ui.available_height())
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                about_page(ui, self.lang());
                                ui.add_space(10.0);
                                if update::ENABLED {
                                    let busy = self.update_rx.is_some()
                                        || !matches!(self.update, UpdateState::Idle);
                                    if ui
                                        .add_enabled(
                                            !busy,
                                            ui_c::secondary_button(self.tr(T::CheckForUpdates)),
                                        )
                                        .clicked()
                                    {
                                        self.skipped_version.clear();
                                        self.checked_manually = true;
                                        self.start_update_check();
                                    }
                                    if self.checked_manually
                                        && matches!(self.update, UpdateState::Idle)
                                        && self.update_rx.is_none()
                                    {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} (v{})",
                                                self.tr(T::OnNewestRelease),
                                                env!("CARGO_PKG_VERSION")
                                            ))
                                            .font(t::plex(12.0))
                                            .color(t::TEXT_MUTED),
                                        );
                                    }
                                } else {
                                    ui.label(
                                        RichText::new(self.tr(T::ForkNoUpdate))
                                            .font(t::plex(12.0))
                                            .color(t::TEXT_MUTED),
                                    );
                                }
                            });
                        return;
                    }
                    Page::Setup => {}
                }
                // Everything below scrolls: on a 768 px-tall screen the button row
                // and the log fell off the bottom with no way to reach them (#64).
                egui::ScrollArea::both()
                    .max_height(ui.available_height())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                // The setup panel was designed at 720 px; keep it from stretching,
                // and give it that width to lay out in even when the window is
                // narrower — the scroll area is what makes the rest reachable (#64).
                ui.set_max_width(860.0);
                ui.set_min_width(720.0);

                // ── path row ──────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let btn_w = 2.0 * 96.0 + 8.0;
                    let field_w = ui.available_width() - btn_w - 8.0;
                    let path_hint = self.tr(T::GameFolderOrExe);
                    let browse_l = self.tr(T::BrowseGameFolder);
                    let pick_title = self.tr(T::PickGameFolder);
                    let game_exe_hint = self.tr(T::GameExe);
                    Frame::new()
                        .fill(t::BG)
                        .stroke(Stroke::new(1.0, t::BORDER_STRONG))
                        .corner_radius(t::control_rounding())
                        .inner_margin(Margin::symmetric(12, 0))
                        .show(ui, |ui| {
                            ui.set_min_height(40.0);
                            ui.set_width(field_w - 24.0);
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                let r = ui.add(
                                    egui::TextEdit::singleline(&mut self.exe_text)
                                        .frame(Frame::NONE)
                                        .font(t::mono(12.0))
                                        .text_color(t::TEXT_SOFT)
                                        .hint_text(RichText::new(path_hint).color(t::TEXT_DIM))
                                        .desired_width(f32::INFINITY),
                                );
                                if r.changed() {
                                    self.refresh();
                                }
                            });
                        });
                    let start_dir = self.exe().and_then(|p| p.parent().map(|d| d.to_path_buf()));
                    if ui
                        .add_sized([96.0, 40.0], egui::Button::new(browse_l))
                        .clicked()
                    {
                        let mut dlg = rfd::FileDialog::new().set_title(pick_title);
                        if let Some(d) = &start_dir {
                            dlg = dlg.set_directory(d);
                        }
                        if let Some(p) = dlg.pick_folder() {
                            self.exe_text = p.to_string_lossy().into_owned();
                            self.refresh();
                        }
                    }
                    if ui
                        .add_sized([96.0, 40.0], egui::Button::new("Exe…"))
                        .on_hover_text(game_exe_hint)
                        .clicked()
                    {
                        let mut dlg =
                            rfd::FileDialog::new().add_filter("Executables", &["exe", "bin"]);
                        if let Some(d) = &start_dir {
                            dlg = dlg.set_directory(d);
                        }
                        if let Some(p) = dlg.pick_file() {
                            self.exe_text = p.to_string_lossy().into_owned();
                            self.refresh();
                        }
                    }
                });

                // ── hero: game / collection ───────────────────────
                if let Some(exe) = self.resolved_exe.clone() {
                    let base = self.input_path();
                    let short = |p: &PathBuf| -> String { game::exe_label(&base, p) };
                    let members = game::collection_members(&self.candidates);
                    let collection = members.len() >= 2;
                    let full_path = exe.display().to_string();
                    let path_trunc = ui_c::truncate_path(&full_path, 72);
                    let game_name = short(&exe);
                    let inspecting_l = self.tr(T::Inspecting);
                    let game_exe_l = self.tr(T::GameExe);
                    let coll_title = self.tr(T::CollectionTitle);
                    let coll_sel = self.tr(T::CollectionSelected);
                    let sel_all = self.tr(T::SelectAll);
                    let sel_none = self.tr(T::SelectNone);

                    ui_c::card_elevated().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&game_name)
                                        .font(t::sora(18.0))
                                        .color(t::TEXT),
                                );
                                ui.label(
                                    RichText::new(&path_trunc)
                                        .font(t::mono(11.5))
                                        .color(t::TEXT_MUTED),
                                )
                                .on_hover_text(&full_path);
                            });
                            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                                if let Some(s) = &ok_status {
                                    if let Some((g, tier)) = &s.gpu {
                                        ui_c::chip(
                                            ui,
                                            &format!("{} ({})", g.name, tier.label()),
                                            ui_c::ChipTone::Neutral,
                                        );
                                    }
                                    ui_c::chip(
                                        ui,
                                        &format!("{}-bit · {}", s.bitness, s.api.label()),
                                        ui_c::ChipTone::Primary,
                                    );
                                }
                            });
                        });

                        if collection {
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(coll_title)
                                    .font(t::plex_semibold(12.5))
                                    .color(t::WARN),
                            );
                            ui.add_space(4.0);
                            for c in &members {
                                let mut on = self.collection_selected.contains(c);
                                let label = short(c);
                                if ui
                                    .checkbox(
                                        &mut on,
                                        RichText::new(label)
                                            .font(t::mono(11.5))
                                            .color(t::TEXT_SOFT),
                                    )
                                    .changed()
                                {
                                    if on {
                                        self.collection_selected.insert(c.clone());
                                        self.resolved_exe = Some(c.clone());
                                        self.inspect_resolved();
                                    } else {
                                        self.collection_selected.remove(c);
                                    }
                                }
                            }
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                if ui.small_button(sel_all).clicked() {
                                    self.collection_selected =
                                        members.iter().cloned().collect();
                                }
                                if ui.small_button(sel_none).clicked() {
                                    self.collection_selected.clear();
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "{} {} / {}",
                                        coll_sel,
                                        self.collection_selected.len(),
                                        members.len()
                                    ))
                                    .font(t::plex(11.5))
                                    .color(t::TEXT_SECONDARY),
                                );
                            });
                        }

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.label(
                                RichText::new(if collection {
                                    inspecting_l
                                } else {
                                    game_exe_l
                                })
                                .font(t::plex(12.0))
                                .color(t::TEXT_MUTED),
                            );
                            if !collection && self.candidates.len() > 1 {
                                let mut pick = exe.clone();
                                egui::ComboBox::from_id_salt("exe_pick")
                                    .selected_text(
                                        RichText::new(short(&pick)).font(t::mono(11.5)),
                                    )
                                    .show_ui(ui, |ui| {
                                        for c in &self.candidates {
                                            ui.selectable_value(&mut pick, c.clone(), short(c));
                                        }
                                    });
                                if pick != exe {
                                    self.resolved_exe = Some(pick);
                                    self.inspect_resolved();
                                }
                            } else {
                                chip(ui, &short(&exe), t::TEXT_SOFT, false);
                            }
                            if let Some(s) = &ok_status {
                                let mut choice = game::mode_override();
                                let name = |m: Option<game::Mode>| match m {
                                    None => match s.mode_detected {
                                        game::Mode::Native => {
                                            "Auto: native DLSS · renodx hooks game (Feeder Optimize N/A)"
                                        }
                                        game::Mode::Feeder => {
                                            "Auto: no DLSS · Feeder path + Optimize"
                                        }
                                    },
                                    Some(game::Mode::Native) => {
                                        "Force native DLSS (Feeder Optimize N/A)"
                                    }
                                    Some(game::Mode::Feeder) => {
                                        "Force no-DLSS (Feeder + Optimize)"
                                    }
                                };
                                let before = choice;
                                egui::ComboBox::from_id_salt("mode_pick")
                                    .selected_text(
                                        RichText::new(name(choice))
                                            .font(t::plex(12.0))
                                            .color(t::TEXT_SOFT),
                                    )
                                    .show_ui(ui, |ui| {
                                        for m in [
                                            None,
                                            Some(game::Mode::Feeder),
                                            Some(game::Mode::Native),
                                        ] {
                                            ui.selectable_value(&mut choice, m, name(m));
                                        }
                                    });
                                if choice != before && !self.running {
                                    game::set_mode_override(choice);
                                    self.inspect_resolved();
                                }
                            }
                        });
                    });
                }

                for p in &problems {
                    ui.label(
                        RichText::new(text::tidy(p))
                            .font(t::plex(12.0))
                            .color(t::DANGER),
                    );
                }
                if ok_status
                    .as_ref()
                    .is_some_and(|s| s.needs_dgvoodoo() && problems.is_empty())
                {
                    ui.label(
                        RichText::new(
                            "DirectX 9: Install will download dgVoodoo 2.87.3 into the game folder \
                             first (official GitHub release → d3d9.dll + dgVoodoo.conf), then continue \
                             with ReShade / Feeder. / DirectX 9: Install сначала скачает dgVoodoo 2.87.3 \
                             в папку игры, затем продолжит установку.",
                        )
                        .font(t::plex(12.0))
                        .color(t::TEXT_SOFT),
                    );
                }
                if let Some(s) = &ok_status {
                    let mut caps: Vec<&str> = Vec::new();
                    if s.mode == game::Mode::Native {
                        caps.push("Native DLSS (Feeder Optimize N/A)");
                    }
                    if s.rt_likely {
                        caps.push("RT-likely");
                    }
                    if s.re_engine {
                        caps.push("RE Engine");
                    }
                    if s.unreal_likely {
                        caps.push("Unreal-likely");
                    }
                    if s.unity_likely {
                        caps.push("Unity-likely");
                    }
                    if s.api == game::Api::Dx12 && s.mode == game::Mode::Feeder {
                        caps.push("DX12 Feeder (Optimize, no OFA)");
                    }
                    if !caps.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            for c in caps {
                                ui_c::chip(ui, c, ui_c::ChipTone::Neutral);
                            }
                        });
                    }
                }
                // Remote Desktop hands the session a virtual display adapter, so the
                // real card is invisible and the check refuses a machine that would
                // work locally (#3). Same shape of escape hatch as the anti-cheat one.
                if self
                    .status
                    .as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .is_some_and(|s| s.problems.iter().any(|p| p.starts_with("GPU is ")))
                    || game::skip_gpu_check()
                {
                    let mut on = game::skip_gpu_check();
                    let cb = egui::Checkbox::new(
                        &mut on,
                        RichText::new(
                            "That is not the card I game on \u{2014} Remote Desktop, a virtual display, or a misread. Check anyway, at my own risk",
                        )
                        .font(t::plex(11.5))
                        .color(t::TEXT_SOFT),
                    );
                    if ui.add_enabled(!self.running, cb).changed() {
                        game::set_skip_gpu_check(on);
                        self.inspect_resolved();
                    }
                }
                if let Some(ac) = ok_status.as_ref().and_then(|s| s.anticheat) {
                    let mut on = game::ignore_anticheat();
                    let label = format!(
                        "{ac} is switched off for offline play in this game (GTA V: BattlEye unticked in the Rockstar Games Launcher, or -nobattleye) — install anyway, at my own risk"
                    );
                    let cb = egui::Checkbox::new(
                        &mut on,
                        RichText::new(label).font(t::plex(11.5)).color(t::TEXT_SOFT),
                    );
                    if ui.add_enabled(!self.running, cb).changed() {
                        game::set_ignore_anticheat(on);
                        self.inspect_resolved();
                    }
                }

                // ── install method ────────────────────────────────
                let native =
                    ok_status.as_ref().is_some_and(|s| s.mode == game::Mode::Native);
                if !native {
                    self.engine = Engine::ReShade;
                }
                let reshade_blocked = ok_status
                    .as_ref()
                    .and_then(game::GameStatus::reshade_engine_problem)
                    .is_some();
                let method_title = self.tr(T::InstallMethod);
                let eng_rs = self.tr(T::EngineReshade);
                let eng_rs_d = self.tr(T::EngineReshadeDetail);
                let eng_op = self.tr(T::EngineOpti);
                let eng_op_d = self.tr(T::EngineOptiDetail);
                let recommended = self.tr(T::Recommended);
                ui_c::section_card(ui, method_title, |ui| {
                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            if ui_c::selectable_option(
                                ui,
                                self.engine == Engine::ReShade,
                                !reshade_blocked,
                                eng_rs,
                                eng_rs_d,
                            )
                            .clicked()
                            {
                                self.engine = Engine::ReShade;
                            }
                            if !reshade_blocked {
                                ui.add_space(4.0);
                                ui_c::chip(ui, recommended, ui_c::ChipTone::Success);
                            }
                        });
                        cols[1].vertical(|ui| {
                            if ui_c::selectable_option(
                                ui,
                                self.engine == Engine::Opti,
                                native,
                                eng_op,
                                eng_op_d,
                            )
                            .clicked()
                            {
                                self.engine = Engine::Opti;
                            }
                        });
                    });
                });

                // ── neural rendering options ──────────────────────
                let nr_title = self.tr(T::NeuralRendering);
                ui_c::section_card(ui, nr_title, |ui| {
                    if self.engine == Engine::ReShade {
                        let mut on = self.renodx_classic;
                        let cb = egui::Checkbox::new(
                            &mut on,
                            RichText::new(
                                "Black screen, crash or driver reset with DLSS 5 on? Install the classic add-on build (4.55)",
                            )
                            .font(t::plex(11.5))
                            .color(t::TEXT_SOFT),
                        );
                        if ui.add_enabled(!self.running, cb).changed() {
                            self.renodx_classic = on;
                        }
                        ui.add_space(8.0);
                        if !native {
                            self.upstream_on = false;
                        }
                        let stable_l = self.tr(T::StableRenodx);
                        let upstream_l = self.tr(T::ExperimentalUpstream);
                        ui.columns(2, |cols| {
                            if ui_c::selectable_option(
                                &mut cols[0],
                                !self.upstream_on,
                                true,
                                stable_l,
                                "The network runs after the upscaler, at output resolution.",
                            )
                            .clicked()
                            {
                                self.upstream_on = false;
                            }
                            if ui_c::selectable_option(
                                &mut cols[1],
                                self.upstream_on,
                                native,
                                upstream_l,
                                "Runs before the upscaler at render resolution — lower cost.",
                            )
                            .clicked()
                            {
                                self.upstream_on = true;
                            }
                        });
                        if self.upstream_on {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                ui.label(
                                    RichText::new("upstream_preset")
                                        .font(t::mono(11.5))
                                        .color(t::TEXT_MUTED),
                                );
                                let current = reshade_ini::UPSTREAM_PRESETS
                                    .iter()
                                    .find(|(_, id, _)| *id == self.upstream_preset)
                                    .map(|(n, ..)| *n)
                                    .unwrap_or("Reference");
                                egui::ComboBox::from_id_salt("upstream_preset")
                                    .selected_text(RichText::new(current).font(t::plex(12.0)))
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        for (name, id, _) in reshade_ini::UPSTREAM_PRESETS {
                                            ui.selectable_value(
                                                &mut self.upstream_preset,
                                                id,
                                                RichText::new(name).font(t::plex(12.0)),
                                            );
                                        }
                                    });
                                ui.label(
                                    RichText::new(match self.upstream_preset {
                                        1 => "Keeps the lighting work, holds back invented detail.",
                                        2 => "Half way: detail is enhanced but not rebuilt.",
                                        4 => "Past what the network intends \u{2014} detail starts looking drawn.",
                                        5 => "Deliberately overcooked: waxy skin, invented surfaces.",
                                        _ => "Everything the network wants to do. Its own default.",
                                    })
                                    .font(t::plex(11.0))
                                    .color(t::TEXT_DIM),
                                );
                            });
                            ui.add_space(6.0);
                            let warn = "EXPERIMENTAL. Using DLSS Frame Generation? Set this add-on to \
                                        Quality in the ReShade overlay, so the network runs on every \
                                        frame. At any lower setting it runs on one frame in two or \
                                        three, the rendered frame interval alternates, and frame \
                                        generation cannot pace through the swing: stutter and flashes \
                                        that get worse the higher the multiplier. Its author tested it \
                                        against GTA V Enhanced and Bright Memory: Infinite only.";
                            Frame::new()
                                .fill(t::WARNING_SOFT)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(0x7a, 0x5a, 0x22)))
                                .corner_radius(t::control_rounding())
                                .inner_margin(Margin::same(10))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(warn)
                                            .font(t::plex(11.0))
                                            .color(Color32::from_rgb(0xe8, 0xc9, 0x8a)),
                                    );
                                });
                        }
                    }

                    if self.engine == Engine::Opti {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.label(
                                RichText::new("opti_presr")
                                    .font(t::mono(11.5))
                                    .color(t::TEXT_MUTED),
                            );
                            for (presr, label) in [
                                (false, "Stable"),
                                (true, "Pre-SR multipass \u{00b7} experimental"),
                            ] {
                                let on = self.opti_presr == presr;
                                let btn = egui::Button::new(
                                    RichText::new(label)
                                        .font(t::plex_medium(12.0))
                                        .color(if on {
                                            Color32::WHITE
                                        } else {
                                            t::TEXT_SOFT
                                        }),
                                )
                                .fill(if on {
                                    t::PRIMARY
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(Stroke::new(
                                    1.0,
                                    if on { t::PRIMARY } else { t::BORDER_STRONG },
                                ))
                                .corner_radius(t::control_rounding())
                                .min_size(Vec2::new(if presr { 200.0 } else { 90.0 }, 30.0));
                                if ui.add(btn).clicked() {
                                    self.opti_presr = presr;
                                }
                            }
                        });
                        ui.label(
                            RichText::new(if self.opti_presr {
                                "wilsjo2's fork: the network runs BEFORE super resolution, on the \
                                 smaller image, in 1-3 passes. Bigger download (about 160 MB) and \
                                 barely tested here \u{2014} report what you see."
                            } else {
                                "Dagherbou's build: the network runs after super resolution. \
                                 The one most people are running."
                            })
                            .font(t::plex(11.0))
                            .color(t::TEXT_DIM),
                        );
                        if self.opti_presr
                            && ok_status
                                .as_ref()
                                .and_then(|s| s.gpu.as_ref())
                                .is_some_and(|(_, t)| *t == crate::gpu::Tier::Rtx40)
                        {
                            ui.add_space(6.0);
                            let mut on = self.ada_mfg;
                            let cb = egui::Checkbox::new(
                                &mut on,
                                RichText::new(
                                    "ada_mfg \u{2014} RTX 40 multi-frame generation. Built into this build, no extra files. The game must have frame generation of its own.",
                                )
                                .font(t::plex(11.5))
                                .color(t::TEXT_SOFT),
                            );
                            if ui.add_enabled(!self.running, cb).changed() {
                                self.ada_mfg = on;
                            }
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.label(
                                RichText::new("working_scale")
                                    .font(t::mono(11.5))
                                    .color(t::TEXT_MUTED),
                            );
                            for (scale, label, note) in [
                                (1.0_f32, "100%", "full cost"),
                                (0.75, "75%", "about half the cost"),
                                (0.5, "50%", "about a quarter"),
                            ] {
                                let on = (self.working_scale - scale).abs() < 0.01;
                                let btn = egui::Button::new(
                                    RichText::new(label)
                                        .font(t::plex_medium(12.0))
                                        .color(if on {
                                            Color32::WHITE
                                        } else {
                                            t::TEXT_SOFT
                                        }),
                                )
                                .fill(if on {
                                    t::PRIMARY
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(Stroke::new(
                                    1.0,
                                    if on { t::PRIMARY } else { t::BORDER_STRONG },
                                ))
                                .corner_radius(t::control_rounding())
                                .min_size(Vec2::new(72.0, 30.0));
                                if ui.add(btn).on_hover_text(note).clicked() {
                                    self.working_scale = scale;
                                }
                            }
                            ui.label(
                                RichText::new(match self.working_scale {
                                    s if s >= 0.99 => {
                                        "The model runs at full output resolution."
                                    }
                                    s if s >= 0.74 => {
                                        "Costs about half as much; the biggest single fps lever here."
                                    }
                                    _ => {
                                        "Costs about a quarter; the model's contribution is softer."
                                    }
                                })
                                .font(t::plex(11.0))
                                .color(t::TEXT_DIM),
                            );
                        });
                    }
                });

                // ── components ────────────────────────────────────
                let components_l = self.tr(T::Components);
                ui_c::section_card(ui, components_l, |ui| {
                    let gap = 24.0;
                    let tile_h = 44.0;
                    let row_w = ui.available_width();
                    let col_w = ((row_w - gap) / 2.0).floor();
                    let tiles =
                        tiles_for(ok_status.as_ref(), self.engine, self.renodx_on, self.upstream_on);
                    for row in tiles.chunks(2) {
                        let (row_rect, _) = ui
                            .allocate_exact_size(Vec2::new(row_w, tile_h), egui::Sense::hover());
                        for (i, tl) in row.iter().enumerate() {
                            let x = row_rect.left() + i as f32 * (col_w + gap);
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(x, row_rect.top()),
                                Vec2::new(col_w, tile_h),
                            );
                            tile(ui, rect, tl, ok_status.as_ref());
                        }
                    }

                    if let Some(s) = &ok_status {
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.label(
                                RichText::new("RENODX HDR MOD")
                                    .font(t::plex_semibold(11.0))
                                    .color(t::TEXT_MUTED),
                            );
                            let dim = |ui: &mut egui::Ui, text: String| {
                                ui.label(
                                    RichText::new(text).font(t::plex(11.0)).color(t::TEXT_DIM),
                                );
                            };
                            if let Some(installed) = &s.renodx_mod {
                                dim(
                                    ui,
                                    format!("— installed: {installed} (Remove takes it out too)"),
                                );
                            } else if !s.foreign_renodx.is_empty() {
                                dim(
                                    ui,
                                    format!(
                                        "— already present, not installed by this tool: {} (left untouched; ReShade loads one RenoDX mod per game)",
                                        s.foreign_renodx.join(", ")
                                    ),
                                );
                            } else {
                                match &self.renodx {
                                    RenodxLookup::Idle | RenodxLookup::Pending => dim(
                                        ui,
                                        "— looking up clshortfuse/renodx for this game…".into(),
                                    ),
                                    RenodxLookup::NotFound => {
                                        dim(ui, "— no RenoDX mod is published for this game.".into())
                                    }
                                    RenodxLookup::Failed(e) => {
                                        dim(ui, format!("— lookup failed: {e}"))
                                    }
                                    RenodxLookup::Found(m) => {
                                        let label = format!(
                                            "Also install {} — {}",
                                            m.file,
                                            m.status_label()
                                        );
                                        let cb = egui::Checkbox::new(
                                            &mut self.renodx_on,
                                            RichText::new(label)
                                                .font(t::plex(12.0))
                                                .color(t::TEXT_SOFT),
                                        );
                                        ui.add_enabled(!self.running, cb).on_hover_text(
                                            "Game-specific HDR / tone-mapping mod from the RenoDX project. Loads beside the DLSS 5 add-on (different add-on name, different settings section). Turn Windows AutoHDR / RTX HDR off to avoid double tone mapping.",
                                        );
                                        if self.engine == Engine::Opti {
                                            dim(
                                                ui,
                                                "— ReShade goes in as ReShade64.dll, loaded by OptiScaler (LoadReshade=true)".into(),
                                            );
                                        }
                                        if !m.note.is_empty() {
                                            dim(ui, format!("— {}", m.note));
                                        }
                                    }
                                }
                            }
                        });
                    }
                });

                // ── actions ───────────────────────────────────────
                let collection_ok = !game::is_collection(&self.candidates)
                    || !self.collection_selected.is_empty();
                let can_run =
                    ok_status.is_some() && problems.is_empty() && !self.running && collection_ok;
                let n_sel = self.install_targets().len();
                let install_label = if game::is_collection(&self.candidates) && n_sel > 1 {
                    fmt_n(self.tr(T::InstallN), n_sel)
                } else {
                    self.tr(T::Install).to_owned()
                };
                let remove_l = self.tr(T::Remove);
                let diagnose_l = self.tr(T::Diagnose);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if ui
                        .add_enabled(can_run, ui_c::primary_button(install_label))
                        .clicked()
                    {
                        self.start(None);
                    }
                    if ui
                        .add_enabled(
                            ok_status.is_some() && !self.running,
                            ui_c::secondary_button(remove_l),
                        )
                        .clicked()
                    {
                        self.confirm_remove = true;
                    }
                    if ui
                        .add_enabled(
                            ok_status.is_some() && !self.running,
                            ui_c::secondary_button(diagnose_l),
                        )
                        .on_hover_text(self.tr(T::DiagnoseHint))
                        .clicked()
                    {
                        self.run_diagnose();
                    }
                    let vulkan = ok_status
                        .as_ref()
                        .is_some_and(|s| s.api == game::Api::Vulkan);
                    if vulkan {
                        if ui
                            .add_enabled(
                                !self.running,
                                ui_c::secondary_button(self.tr(T::CopyVulkanKit)),
                            )
                            .on_hover_text(self.tr(T::CopyVulkanKitHint))
                            .clicked()
                        {
                            if let Some(exe) = self.exe() {
                                let dir = exe.parent().unwrap().to_path_buf();
                                match installer::copy_vulkan_feeder_kit(&dir) {
                                    Ok(files) => {
                                        self.log.push(LogLine::Ok(format!(
                                            "Vulkan kit: {}",
                                            files.join(", ")
                                        )));
                                        self.inspect_resolved();
                                    }
                                    Err(e) => {
                                        self.log.push(LogLine::Fail(format!("{e:#}")));
                                        self.last_error = Some(format!("{e:#}"));
                                    }
                                }
                            }
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let missing = ok_status
                            .as_ref()
                            .map(installer::missing_install_files)
                            .unwrap_or_default();
                        let stale = self
                            .exe()
                            .and_then(|e| {
                                self.meta
                                    .values()
                                    .find(|m| m.exe == e)
                                    .map(|m| m.stale.clone())
                            })
                            .unwrap_or_default();
                        let msg = if self.running || !self.progress_msg.is_empty() {
                            self.progress_msg.clone()
                        } else if !missing.is_empty() {
                            format!("Incomplete: missing {}", missing.join(", "))
                        } else if !stale.is_empty() {
                            format!("Update available: {}", stale.join("; "))
                        } else if complete {
                            "Everything is in place.".to_owned()
                        } else {
                            String::new()
                        };
                        let color = if msg.starts_with("Failed") || msg.starts_with("Incomplete")
                        {
                            t::DANGER
                        } else if msg.starts_with("Update available") {
                            t::WARN
                        } else {
                            t::SUCCESS
                        };
                        ui.label(RichText::new(msg).font(t::plex_medium(12.0)).color(color));
                    });
                });

                // Offline knobs / expected FPS from Feeder perf log.
                let wide_bottom = ui.available_width() >= 1080.0;
                if wide_bottom {
                    ui.columns(2, |cols| {
                        self.knobs_panel(&mut cols[0]);
                        self.hotkeys_panel(&mut cols[1]);
                    });
                } else {
                    self.knobs_panel(ui);
                    self.hotkeys_panel(ui);
                }

                // ── progress ──────────────────────────────────────
                let (bar, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 4.0),
                    egui::Sense::hover(),
                );
                ui.painter()
                    .rect_filled(bar, CornerRadius::same(2), t::BORDER);
                let frac = if self.running {
                    self.progress as f32 / 100.0
                } else if self.progress == 100 || complete {
                    1.0
                } else {
                    0.0
                };
                if frac > 0.0 {
                    let mut fill = bar;
                    fill.set_width(bar.width() * frac);
                    ui.painter()
                        .rect_filled(fill, CornerRadius::same(2), t::PRIMARY);
                }

                // ── install log ───────────────────────────────────
                let log_title = self.tr(T::InstallLog);
                let hint_h = 34.0;
                let fails: Vec<&LogLine> = self
                    .log
                    .iter()
                    .filter(|l| matches!(l, LogLine::Fail(_)))
                    .collect();
                let oks = self
                    .log
                    .iter()
                    .filter(|l| matches!(l, LogLine::Ok(_) | LogLine::Step(_)))
                    .count();
                let summary = if self.log.is_empty() {
                    format!("{log_title} — empty")
                } else if !fails.is_empty() {
                    format!("{log_title} — {} error(s), {oks} step(s)", fails.len())
                } else {
                    format!("{log_title} — {} line(s)", self.log.len())
                };
                ui_c::card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let arrow = if self.log_expanded { "▾" } else { "▸" };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("{arrow} {summary}"))
                                        .font(t::plex_medium(12.0))
                                        .color(if fails.is_empty() {
                                            t::TEXT_MUTED
                                        } else {
                                            t::DANGER
                                        }),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            self.log_expanded = !self.log_expanded;
                        }
                    });
                    if self.log_expanded {
                        let log_h = (ui.available_height() - hint_h - 12.0).max(80.0);
                        Frame::new()
                            .fill(t::BG)
                            .stroke(Stroke::new(1.0, t::BORDER))
                            .corner_radius(t::control_rounding())
                            .inner_margin(Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_height(log_h - 20.0);
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(fails.is_empty())
                                    .show(ui, |ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.set_width(ui.available_width());
                                        let mut ordered: Vec<&LogLine> =
                                            Vec::with_capacity(self.log.len());
                                        ordered.extend(
                                            self.log
                                                .iter()
                                                .filter(|l| matches!(l, LogLine::Fail(_))),
                                        );
                                        ordered.extend(
                                            self.log
                                                .iter()
                                                .filter(|l| !matches!(l, LogLine::Fail(_))),
                                        );
                                        for l in ordered {
                                            match l {
                                                LogLine::Step(s) => {
                                                    let (idx, rest) =
                                                        s.split_once(' ').unwrap_or(("", s));
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 6.0;
                                                        ui.label(
                                                            RichText::new(idx)
                                                                .font(t::mono(11.5))
                                                                .color(t::TEXT_DIM),
                                                        );
                                                        ui.label(
                                                            RichText::new(rest)
                                                                .font(t::mono(11.5))
                                                                .color(t::TEXT_MUTED),
                                                        );
                                                    });
                                                }
                                                LogLine::Ok(s) => {
                                                    ui.label(
                                                        RichText::new(format!("      {s}"))
                                                            .font(t::mono(11.5))
                                                            .color(t::SUCCESS),
                                                    );
                                                }
                                                LogLine::Fail(s) => {
                                                    ui.label(
                                                        RichText::new(format!("      {s}"))
                                                            .font(t::mono(11.5))
                                                            .color(t::DANGER),
                                                    );
                                                }
                                                LogLine::Plain(s) => {
                                                    ui.label(
                                                        RichText::new(s)
                                                            .font(t::mono(11.5))
                                                            .color(t::TEXT),
                                                    );
                                                }
                                            }
                                        }
                                    });
                            });
                    } else if !fails.is_empty() {
                        ui.label(
                            RichText::new(
                                fails
                                    .iter()
                                    .filter_map(|l| match l {
                                        LogLine::Fail(s) => Some(s.as_str()),
                                        _ => None,
                                    })
                                    .take(2)
                                    .collect::<Vec<_>>()
                                    .join(" · "),
                            )
                            .font(t::mono(11.0))
                            .color(t::DANGER),
                        );
                    }
                });

                ui.label(
                    RichText::new(
                        "After install, in game: press Home for the ReShade overlay, open the DLSS 5 Neural Rendering panel and enable it. \
                         Keep MSAA/SSAA off. Check dlss5-feed.log next to the exe for 'feature ready'. \
                         Optional TRAA (LUMENITE: TRAA): keep it BELOW DLSS 5 Feed; Edge Detection=Geometric + Protect UI/text on — \
                         re-run install to patch TRAA if UI still smears.",
                    )
                    .font(t::plex(11.0))
                    .color(t::TEXT_DIM),
                );
                    });
                });
            });

        // ── dialogs ───────────────────────────────────────────────
        if self.confirm_remove {
            let confirm_q = self.tr(T::ConfirmRemove);
            let confirm_all = self.tr(T::ConfirmRemoveAll);
            let remove_l = self.tr(T::Remove);
            let cancel_l = self.tr(T::Cancel);
            egui::Window::new(RichText::new(remove_l).font(t::sora(14.0)))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.set_max_width(420.0);
                    ui.label(RichText::new(confirm_q).font(t::plex(13.0)).color(t::TEXT));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(confirm_all)
                            .font(t::plex(12.0))
                            .color(t::TEXT_MUTED),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let danger = egui::Button::new(
                            RichText::new(remove_l)
                                .font(t::plex_semibold(13.0))
                                .color(Color32::WHITE),
                        )
                        .fill(t::DANGER)
                        .stroke(Stroke::NONE)
                        .corner_radius(t::control_rounding())
                        .min_size(Vec2::new(100.0, 36.0));
                        if ui.add(danger).clicked() {
                            self.confirm_remove = false;
                            self.start(Some(false));
                        }
                        if ui
                            .add(
                                ui_c::secondary_button(confirm_all)
                                    .min_size(Vec2::new(160.0, 36.0)),
                            )
                            .clicked()
                        {
                            self.confirm_remove = false;
                            self.start(Some(true));
                        }
                        if ui
                            .add(ui_c::secondary_button(cancel_l).min_size(Vec2::new(90.0, 36.0)))
                            .clicked()
                        {
                            self.confirm_remove = false;
                        }
                    });
                });
        }
        if let Some(err) = self.last_error.clone() {
            egui::Window::new(RichText::new("DLSS5oneclick").font(t::sora(14.0)))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.set_max_width(480.0);
                    ui.label(RichText::new(&err).color(t::DANGER));
                    if ui.button(self.tr(T::Ok)).clicked() {
                        self.last_error = None;
                    }
                });
        }
    }
}

pub fn run() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 780.0])
        // A 1366x768 screen has ~730 px of usable height once the taskbar and the
        // title bar are gone, so a 620 px floor left the window unshrinkable there
        // and the buttons unreachable (#64). Everything scrolls now, so this can go
        // as small as the layout itself needs.
        .with_min_inner_size([640.0, 420.0])
        .with_title(concat!("DLSS5oneclick ", env!("CARGO_PKG_VERSION")));
    if let Some(icon) = logo::icon_data() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        concat!("DLSS5oneclick ", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{stub_status, Api, Mode};

    /// A long game title used to wrap and then get sliced by the caption's
    /// clip rect, so it read as cut off mid-word. Shorten it honestly (#77).
    #[test]
    fn a_long_title_is_shortened_with_an_ellipsis() {
        // Fake metrics: every character is 10 wide.
        let measure = |t: &str| t.chars().count() as f32 * 10.0;

        assert_eq!(truncate_to_fit("Spore", 100.0, measure), "Spore");

        let long = "The Witcher 3: Wild Hunt - Game of the Year Edition";
        let out = truncate_to_fit(long, 100.0, measure);
        assert!(out.ends_with('\u{2026}'), "{out}");
        assert!(measure(&out) <= 100.0, "{out}");
        assert!(long.starts_with(out.trim_end_matches('\u{2026}')), "{out}");

        // Multi-byte titles must not be cut through a character.
        let jp = "ファイナルファンタジー";
        let out = truncate_to_fit(jp, 45.0, measure);
        assert!(measure(&out) <= 45.0, "{out}");
        assert!(jp.starts_with(out.trim_end_matches('\u{2026}')), "{out}");
    }

    /// A finished OptiScaler install must not show a missing row: OptiScaler
    /// carries its own neural pass, so `renodx-dlss5.addon64` is never
    /// installed on that route and asking for it contradicted `complete()` (#66).
    #[test]
    fn opti_route_tiles_agree_with_complete() {
        let mut st = stub_status(Mode::Native, Api::Dx11);
        st.opti = true;
        st.dlssnr = true;
        assert!(st.complete());
        let tiles = tiles_for(Some(&st), Engine::Opti, false, false);
        let missing: Vec<&str> = tiles
            .iter()
            .filter(|t| !(t.ok)(&st) && !t.optional)
            .map(|t| t.title)
            .collect();
        assert!(missing.is_empty(), "shown as missing: {missing:?}");
    }

    /// The ReShade route still wants both files, and still says so when the
    /// add-on is absent.
    #[test]
    fn reshade_route_still_wants_the_addon() {
        let mut st = stub_status(Mode::Native, Api::Dx11);
        st.reshade = true;
        st.dlssnr = true;
        let tiles = tiles_for(Some(&st), Engine::ReShade, false, false);
        assert!(tiles
            .iter()
            .any(|t| t.title == "DLSS 5 add-on \u{00b7} leaked" && !(t.ok)(&st)));
    }
}

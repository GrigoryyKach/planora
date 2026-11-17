//! The application: state, the per-frame logic, navigation and shared operations.
//! Views live in the sibling modules.

use std::path::PathBuf;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Weekday};
use eframe::egui::{self, Frame, Id, Key, Margin, Modifiers, Rect, Ui};

use crate::bus::{self, Cmd};
use crate::config::{Config, Nav, State, Window};
use crate::i18n::{self, Lang};
use crate::model::calendar::{self, ItemKind};
use crate::model::reminders::{missed_reminders, Reminder};
use crate::model::storage::{self, LoadOutcome};
use crate::model::{new_id, Data, Priority, Profile, ProfileFilter, Project, Status, Task};
use crate::nimvoro::Updater;
use crate::notify::{self, Worker};
use crate::platform;
use crate::strings::{self, S};
use crate::theme::{self, Pal, Textures};
use crate::tray::Tray;

mod calendar_view;
mod dialogs;
mod editor;
mod forms;
mod list_view;
mod overlays;
mod settings_view;
mod sidebar;
mod today_view;
pub mod widgets;

pub use dialogs::Dialog;

pub const ICON_PNG: &[u8] = include_bytes!("../../assets/icon.png");
/// Colours offered for projects and profiles.
pub const PALETTE: [u32; 10] = [0x4f6bff, 0x8a5bff, 0xec4899, 0xef4444, 0xf59e0b, 0xeab308, 0x10b981, 0x06b6d4, 0x64748b, 0x78716c];

/// Where a date picked in the popup goes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DateTarget {
    Deadline(String),
    Scheduled(String),
    Until(String),
    QuickAdd,
}

pub struct Picker {
    pub target: DateTarget,
    pub anchor: egui::Pos2,
    pub month: (i32, u32),
    pub current: Option<NaiveDate>,
}

pub struct Drag {
    pub task_id: String,
    pub kind: ItemKind,
    pub from: NaiveDate,
}

pub enum Undo {
    Delete { task: Box<Task>, index: usize },
}

pub struct Toast {
    pub text: String,
    pub undo: Option<Undo>,
    pub until: f64,
}

#[derive(Default, Clone)]
pub struct ListFilter {
    pub text: String,
    /// Indexes: 0 none, 1 todo, 2 in progress, 3 done, 4 cancelled. Empty = all.
    pub status: [bool; 5],
    /// Indexes: 0 none, 1 low .. 4 urgent. Empty = all.
    pub priority: [bool; 5],
    /// 0 any, 1 overdue, 2 today, 3 this week, 4 none.
    pub deadline: usize,
}

pub struct App {
    pub cfg: Config,
    pub lang: Lang,
    pub dir: PathBuf,
    pub data: Data,
    pub tex: Textures,
    pub updater: Updater,
    pub worker: Worker,
    pub tray: Option<Tray>,
    pub today: NaiveDate,
    pub now: NaiveDateTime,
    pub cal: (i32, u32),
    pub drag: Option<Drag>,
    pub day_panel: Option<NaiveDate>,
    pub editor: Option<editor::Editor>,
    pub selected: Option<String>,
    pub dialog: Option<Dialog>,
    pub picker: Option<Picker>,
    pub toast: Option<Toast>,
    pub filter: ListFilter,
    pub show_archived: bool,
    pub autostart: bool,
    pub settings_msg: Option<String>,
    pub backups: Vec<(NaiveDate, PathBuf)>,
    pub sidebar_drag: Option<String>,
    data_dirty: Option<f64>,
    cfg_dirty: Option<f64>,
    quitting: bool,
    last_nav: Nav,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, mut cfg: Config, dir: PathBuf) -> Self {
        theme::install(&cc.egui_ctx, cfg.theme);
        let lang = i18n::resolve(cfg.language.as_deref());
        let mut cfg_dirty = None;
        if cfg.language.as_deref() != Some(lang.code()) {
            cfg.language = Some(lang.code().to_string());
            cfg_dirty = Some(0.0);
        }
        let now = Local::now().naive_local();
        let (data, outcome) = storage::load(&dir, &now.format("%Y%m%d-%H%M%S").to_string());
        bus::init(cc.egui_ctx.clone());

        // The toast icon must be a real file.
        let icon_path = dir.join("icon.png");
        let _ = std::fs::create_dir_all(&dir);
        if std::fs::read(&icon_path).map(|b| b != ICON_PNG).unwrap_or(true) {
            let _ = std::fs::write(&icon_path, ICON_PNG);
        }
        notify::register_identity(&icon_path);

        let state = State::load_from(&dir);
        let missed: Vec<Reminder> = match state.last_tick {
            Some(last) if cfg.notifications => missed_reminders(&data.tasks, cfg.default_remind, last, now, 14),
            _ => Vec::new(),
        };
        let worker = Worker::start(dir.clone(), icon_path, lang);
        worker.update(&data.tasks, cfg.default_remind, cfg.notifications, lang, cfg.time_24h);

        let tray = Tray::new(lang, ICON_PNG);
        {
            let ctx = cc.egui_ctx.clone();
            platform::watch_for_second_launch(move || {
                bus::push(Cmd::Show);
                ctx.request_repaint();
            });
        }

        let updater = Updater::new("planora", crate::REPO_URL.trim_start_matches("https://github.com/"), env!("CARGO_PKG_VERSION"));
        if cfg.check_updates {
            let ctx = cc.egui_ctx.clone();
            updater.check(move || ctx.request_repaint());
        }

        let mut dialog = None;
        if let LoadOutcome::Recovered { moved_to } = outcome {
            dialog = Some(Dialog::Corrupt(moved_to));
        } else if !missed.is_empty() {
            dialog = Some(Dialog::Missed(missed));
        }
        // A project that no longer exists falls back to Today.
        if let Nav::Project(id) = &cfg.nav {
            if data.project(id).is_none() {
                cfg.nav = Nav::Today;
            }
        }
        let today = now.date();
        let last_nav = cfg.nav.clone();
        Self {
            lang,
            tex: Textures::new(&cc.egui_ctx),
            updater,
            worker,
            tray,
            today,
            now,
            cal: (today.year(), today.month()),
            drag: None,
            day_panel: None,
            editor: None,
            selected: None,
            dialog,
            picker: None,
            toast: None,
            filter: ListFilter::default(),
            show_archived: false,
            autostart: platform::autostart_enabled(),
            settings_msg: None,
            backups: storage::list_backups(&dir),
            sidebar_drag: None,
            data_dirty: None,
            cfg_dirty,
            quitting: false,
            last_nav,
            cfg,
            dir,
            data,
        }
    }

    pub fn s(&self) -> &'static S {
        strings::TABLE.get(self.lang)
    }

    pub fn week_start(&self) -> Weekday {
        calendar::week_start_from(self.cfg.week_starts_sunday && self.lang == Lang::En)
    }

    pub fn h24(&self) -> bool {
        self.cfg.time_24h || self.lang != Lang::En
    }

    pub fn color_of(&self, p: Profile) -> egui::Color32 {
        widgets::rgb(self.cfg.profile_color(p))
    }

    // ───────────────────────── persistence ─────────────────────────

    pub fn touch_data(&mut self, ctx: &egui::Context) {
        self.data_dirty = Some(ctx.input(|i| i.time));
        self.worker.update(&self.data.tasks, self.cfg.default_remind, self.cfg.notifications, self.lang, self.h24());
        ctx.request_repaint_after(Duration::from_millis(450));
    }

    pub fn touch_cfg(&mut self, ctx: &egui::Context) {
        self.cfg_dirty = Some(ctx.input(|i| i.time));
        ctx.request_repaint_after(Duration::from_millis(450));
    }

    /// Re-sends the settings to the reminder thread (after a settings change).
    pub fn sync_worker(&self) {
        self.worker.update(&self.data.tasks, self.cfg.default_remind, self.cfg.notifications, self.lang, self.h24());
    }

    pub fn flush(&mut self) {
        if self.data_dirty.take().is_some() {
            if storage::save(&self.dir, &self.data, self.today).is_err() {
                self.toast_text_now(self.s().save_failed.to_string());
            }
            self.backups = storage::list_backups(&self.dir);
        }
        if self.cfg_dirty.take().is_some() {
            self.cfg.save_to(&self.dir);
        }
    }

    fn toast_text_now(&mut self, text: String) {
        self.toast = Some(Toast { text, undo: None, until: f64::MAX });
    }

    pub fn notify(&mut self, ctx: &egui::Context, text: String) {
        let now = ctx.input(|i| i.time);
        self.toast = Some(Toast { text, undo: None, until: now + 4.0 });
    }

    // ───────────────────────── operations ─────────────────────────

    pub fn add_task(&mut self, ctx: &egui::Context, profile: Profile, project: Option<String>, title: &str, date: Option<NaiveDate>) -> String {
        let mut t = Task::new(profile, title, self.now);
        t.project_id = project;
        t.scheduled = date;
        let id = t.id.clone();
        self.data.tasks.push(t);
        self.touch_data(ctx);
        id
    }

    pub fn edit_task(&mut self, ctx: &egui::Context, id: &str, f: impl FnOnce(&mut Task)) {
        let now = self.now;
        if let Some(t) = self.data.task_mut(id) {
            f(t);
            t.updated = now;
            // A time without a day makes no sense.
            if t.scheduled.is_none() {
                t.time = None;
            }
            self.touch_data(ctx);
        }
    }

    pub fn delete_task(&mut self, ctx: &egui::Context, id: &str) {
        let Some(index) = self.data.tasks.iter().position(|t| t.id == id) else { return };
        let task = self.data.tasks.remove(index);
        if self.editor.as_ref().is_some_and(|e| e.id == id) {
            self.editor = None;
        }
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
        }
        let now = ctx.input(|i| i.time);
        self.toast = Some(Toast { text: self.s().task_deleted.to_string(), undo: Some(Undo::Delete { task: Box::new(task), index }), until: now + 8.0 });
        self.touch_data(ctx);
    }

    pub fn undo(&mut self, ctx: &egui::Context) {
        if let Some(Toast { undo: Some(Undo::Delete { task, index }), .. }) = self.toast.take() {
            let i = index.min(self.data.tasks.len());
            self.data.tasks.insert(i, *task);
            self.touch_data(ctx);
        }
    }

    pub fn duplicate_task(&mut self, ctx: &egui::Context, id: &str) {
        let Some(index) = self.data.tasks.iter().position(|t| t.id == id) else { return };
        let mut copy = self.data.tasks[index].clone();
        copy.id = new_id();
        copy.created = self.now;
        copy.updated = self.now;
        copy.completed_at = None;
        copy.status = None;
        copy.done_dates.clear();
        for c in &mut copy.checklist {
            c.done = false;
        }
        copy.title = format!("{} {}", copy.title, self.s().copy_suffix);
        self.data.tasks.insert(index + 1, copy);
        self.touch_data(ctx);
    }

    /// Moves a task one step up or down among the tasks shown (manual sort order).
    pub fn move_task(&mut self, ctx: &egui::Context, id: &str, neighbour: &str) {
        let (Some(a), Some(b)) = (
            self.data.tasks.iter().position(|t| t.id == id),
            self.data.tasks.iter().position(|t| t.id == neighbour),
        ) else {
            return;
        };
        self.data.tasks.swap(a, b);
        self.touch_data(ctx);
    }

    /// Moves a task to another profile/project.
    pub fn move_task_to(&mut self, ctx: &egui::Context, id: &str, profile: Profile, project: Option<String>) {
        self.edit_task(ctx, id, |t| {
            t.profile = profile;
            t.project_id = project;
        });
    }

    pub fn toggle_done(&mut self, ctx: &egui::Context, id: &str, occ: Option<NaiveDate>) {
        let now = self.now;
        if let Some(t) = self.data.task_mut(id) {
            calendar::toggle_done(t, occ, now);
            self.touch_data(ctx);
        }
    }

    pub fn cycle_status(&mut self, ctx: &egui::Context, id: &str) {
        let today = self.today;
        let now = self.now;
        let Some(t) = self.data.task_mut(id) else { return };
        if t.recurrence.is_some() {
            let occ = calendar::current_occurrence(t, today);
            calendar::toggle_done(t, occ, now);
        } else {
            let next = Status::cycle(t.status);
            calendar::set_status(t, next, now);
        }
        self.touch_data(ctx);
    }

    pub fn set_priority(&mut self, ctx: &egui::Context, id: &str, p: Option<Priority>) {
        self.edit_task(ctx, id, |t| t.priority = p);
    }

    pub fn open_editor(&mut self, id: &str) {
        if self.data.task(id).is_some() {
            self.editor = Some(editor::Editor::new(id.to_string()));
            self.selected = Some(id.to_string());
            self.day_panel = None;
        }
    }

    pub fn go(&mut self, ctx: &egui::Context, nav: Nav) {
        if self.cfg.nav != nav {
            self.cfg.nav = nav;
            self.filter = ListFilter::default();
            self.touch_cfg(ctx);
        }
    }

    pub fn project_name(&self, id: &str) -> Option<&str> {
        self.data.project(id).map(|p| p.name.as_str())
    }

    pub fn profile_name(&self, p: Profile) -> &'static str {
        match p {
            Profile::Personal => self.s().personal,
            Profile::Work => self.s().work,
        }
    }

    pub fn new_project(&mut self, ctx: &egui::Context, profile: Profile, name: &str, description: &str, color: u32) -> String {
        let p = Project {
            id: new_id(),
            profile,
            name: name.trim().to_string(),
            description: description.trim().to_string(),
            color,
            archived: false,
            created: self.now,
        };
        let id = p.id.clone();
        self.data.projects.push(p);
        self.touch_data(ctx);
        id
    }

    /// Open tasks with a deadline in the past, for a project or the inbox.
    pub fn overdue_count(&self, profile: Profile, project: Option<&str>) -> usize {
        self.data
            .tasks
            .iter()
            .filter(|t| t.profile == profile && t.project_id.as_deref() == project && calendar::is_overdue(t, self.today))
            .count()
    }

    /// Open (not closed) tasks of a project, or of the inbox.
    pub fn open_count(&self, profile: Profile, project: Option<&str>) -> usize {
        self.data.tasks.iter().filter(|t| t.profile == profile && t.project_id.as_deref() == project && !t.is_closed()).count()
    }

    fn handle_commands(&mut self, ctx: &egui::Context) {
        for cmd in bus::drain() {
            match cmd {
                Cmd::Show => {}
                Cmd::OpenTask(id) => self.open_editor(&id),
                Cmd::NewTask => self.open_quick_add(None),
                Cmd::GoToday => {
                    let n = Nav::Today;
                    self.go(ctx, n);
                }
                Cmd::Quit => self.quit(ctx),
            }
        }
    }

    pub fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        self.flush();
        if let Some(t) = &self.tray {
            t.remove();
        }
        self.worker.stop();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    // ───────────────────────── frame ─────────────────────────

    fn track_window(&mut self, ctx: &egui::Context) {
        let (inner, outer, minimized, maximized) =
            ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect, i.viewport().minimized, i.viewport().maximized));
        if minimized == Some(true) || maximized == Some(true) || !platform::main_window_visible() {
            return;
        }
        if let (Some(inner), Some(outer)) = (inner, outer) {
            let w = Window { w: inner.width(), h: inner.height(), x: Some(outer.min.x), y: Some(outer.min.y) };
            let same = self.cfg.window.is_some_and(|c| {
                (c.w - w.w).abs() < 1.0 && (c.h - w.h).abs() < 1.0 && c.x == w.x && c.y == w.y
            });
            if !same && w.w > 300.0 && w.h > 300.0 {
                self.cfg.window = Some(w);
                self.touch_cfg(ctx);
            }
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let none = Modifiers::NONE;
        let ctrl = Modifiers::COMMAND;
        if ctx.input_mut(|i| i.consume_key(ctrl, Key::N)) {
            self.open_quick_add(None);
        }
        if ctx.input_mut(|i| i.consume_key(ctrl, Key::F)) {
            self.dialog = Some(Dialog::Search { query: String::new(), focus: true });
        }
        if ctx.input_mut(|i| i.consume_key(ctrl, Key::Num1)) {
            self.go(ctx, Nav::Today);
        }
        if ctx.input_mut(|i| i.consume_key(ctrl, Key::Num2)) {
            self.go(ctx, Nav::Calendar);
        }
        if self.editor.is_some() && ctx.input_mut(|i| i.consume_key(ctrl, Key::Enter)) {
            self.flush();
            self.editor = None;
        }
        // Escape closes the top-most thing.
        if ctx.input_mut(|i| i.consume_key(none, Key::Escape)) {
            if self.picker.is_some() {
                self.picker = None;
            } else if self.dialog.is_some() {
                self.dialog = None;
            } else if self.day_panel.is_some() {
                self.day_panel = None;
            } else if self.editor.is_some() {
                self.editor = None;
            } else if self.drag.is_some() {
                self.drag = None;
            }
        }
        if !ctx.egui_wants_keyboard_input() && self.dialog.is_none() {
            if self.cfg.nav == Nav::Calendar {
                if ctx.input_mut(|i| i.consume_key(none, Key::PageUp)) {
                    self.cal = calendar::shift_month(self.cal.0, self.cal.1, -1);
                }
                if ctx.input_mut(|i| i.consume_key(none, Key::PageDown)) {
                    self.cal = calendar::shift_month(self.cal.0, self.cal.1, 1);
                }
            }
            if ctx.input_mut(|i| i.consume_key(none, Key::Delete)) {
                if let Some(id) = self.selected.clone() {
                    self.delete_task(ctx, &id);
                }
            }
        }
    }

    pub fn open_quick_add(&mut self, date: Option<NaiveDate>) {
        let (profile, project) = match &self.cfg.nav {
            Nav::Inbox(p) => (*p, None),
            Nav::Project(id) => match self.data.project(id) {
                Some(pr) => (pr.profile, Some(pr.id.clone())),
                None => (Profile::Personal, None),
            },
            _ => (
                match self.cfg.filter {
                    ProfileFilter::Work => Profile::Work,
                    _ => Profile::Personal,
                },
                None,
            ),
        };
        self.dialog = Some(Dialog::QuickAdd { title: String::new(), profile, project, date, focus: true });
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.now = Local::now().naive_local();
        self.today = self.now.date();
        self.handle_commands(ctx);

        // Close button: hide to the tray instead of quitting.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            if self.cfg.close_to_tray && self.tray.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.flush();
                platform::hide_main_window();
            } else {
                self.quitting = true;
                self.flush();
                self.worker.stop();
            }
        }

        let t = ctx.input(|i| i.time);
        if self.data_dirty.is_some_and(|d| t - d > 0.4) || self.cfg_dirty.is_some_and(|d| t - d > 0.4) {
            self.flush();
        }
        if let Some(toast) = &self.toast {
            if t > toast.until {
                self.toast = None;
            }
        }
        if self.cfg.nav != self.last_nav {
            self.last_nav = self.cfg.nav.clone();
            self.selected = None;
            self.picker = None;
        }
        if platform::main_window_visible() {
            self.track_window(ctx);
            // Keep "today" and the clock fresh without a busy loop.
            ctx.request_repaint_after(Duration::from_secs(30));
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let p = Pal::of(ui.visuals().dark_mode);
        self.handle_keys(&ctx);

        egui::Panel::left("sidebar")
            .exact_size(if self.cfg.sidebar_collapsed { 68.0 } else { 236.0 })
            .resizable(false)
            .frame(Frame::new().fill(p.sidebar).inner_margin(if self.cfg.sidebar_collapsed {
                Margin { left: 10, right: 10, top: 18, bottom: 12 }
            } else {
                Margin { left: 14, right: 12, top: 18, bottom: 12 }
            }))
            .show(ui, |ui| self.sidebar(ui, &p));

        if self.editor.is_some() {
            egui::Panel::right("editor")
                .exact_size(400.0)
                .resizable(false)
                .frame(Frame::new().fill(p.sidebar).inner_margin(Margin { left: 20, right: 14, top: 18, bottom: 12 }))
                .show(ui, |ui| self.editor_panel(ui, &p));
        }

        egui::CentralPanel::default().frame(Frame::new().fill(p.bg).inner_margin(Margin { left: 28, right: 28, top: 20, bottom: 16 })).show(ui, |ui| {
            match self.cfg.nav.clone() {
                Nav::Today => self.today_view(ui, &p),
                Nav::Calendar => self.calendar_view(ui, &p),
                Nav::Inbox(profile) => self.list_view(ui, &p, list_view::Scope::Inbox(profile)),
                Nav::Project(id) => self.list_view(ui, &p, list_view::Scope::Project(id)),
                Nav::Settings => self.settings_view(ui, &p),
            }
        });

        self.day_panel_ui(&ctx, &p);
        self.dialog_ui(&ctx, &p);
        self.picker_ui(&ctx, &p);
        self.toast_ui(&ctx, &p);
        self.drag_ghost(&ctx, &p);
        // Ensure the whole window clears the drag when the pointer is released outside any cell.
        if self.drag.is_some() && ctx.input(|i| i.pointer.any_released()) {
            self.drag = None;
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.flush();
        self.worker.stop();
        if let Some(t) = &self.tray {
            t.remove();
        }
    }
}

/// Rect helper for popups placed under a response.
pub fn below(r: Rect) -> egui::Pos2 {
    egui::pos2(r.left(), r.bottom() + 4.0)
}

pub fn id_of(s: &str) -> Id {
    Id::unique(("planora", s))
}

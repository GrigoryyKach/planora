# Planora

A calm, local planner for tasks, projects and your calendar. It works fully offline, with no account and no cloud. Part of the [NIMVORO](https://github.com/GrigoryyKach/nimvoro) suite.

![Calendar](assets/screenshots/1.png)

## Features

- **Two profiles, one calendar.** *Personal* and *Work* each have an Inbox (tasks without a project) and projects. The calendar shows both, and a filter switches between All, Personal and Work.
- **Month calendar.** Square day cells with chips for planned tasks and deadlines. Overdue deadlines are red, done tasks are struck through, today is highlighted. Click a day for its list, click a chip to edit, drag a chip to another day to reschedule it.
- **Today view.** Today's items, overdue items and what is coming in the next week.
- **Tasks.** Only the title is required. Optional: description, links (with Open and Copy), status, priority, deadline, a planned day with an exact time, a checklist (`2/5` progress), repeat and a reminder.
- **Projects.** Progress card, colour, archive. Lists have filters (status, priority, deadline, text), sorting (manual, deadline, priority, newest) and quick actions on every row: click the status to cycle it, right-click to set status, priority, deadline or plan, to move, duplicate or delete. Deleting offers Undo.
- **Repeats.** Daily, weekdays, weekly, monthly (the 31st becomes the last day of shorter months) or every N days, with an optional end date. Completing one occurrence does not complete the series.
- **Reminders.** A Windows notification at the task time, or 5, 10, 15 or 30 minutes before (a per-task choice, with a default in Settings). Planora stays in the tray when you close the window, and starts minimised to the tray with Windows. Reminders missed while it was off appear once in a "Missed reminders" list. Clicking a notification opens the task.
- **Export to PDF.** A project, the Inbox, the Today view or a single day (from the day panel) becomes a simple A4 task list: a header, then one task per line with a checkbox and the time. Filters such as "Hide done" are respected. Latin and Cyrillic render correctly.
- **Search.** `Ctrl+F` searches all tasks by title and description.
- **Backup.** Autosave, one automatic backup per day (the last 14 are kept), JSON export and import, and restore from the backup list in Settings.

![Project view](assets/screenshots/2.png)
![Task editor](assets/screenshots/3.png)

## Keyboard shortcuts

| Keys | Action |
|---|---|
| `Ctrl+N` | Quick add a task |
| `Ctrl+F` | Search |
| `Ctrl+1` / `Ctrl+2` | Today / Calendar |
| `PageUp` / `PageDown` | Previous / next month (in the calendar) |
| `Delete` | Delete the selected task (with Undo) |
| `Esc` | Close the editor, dialog or picker |
| `Ctrl+Enter` | Save and close the editor |

## Navigation

The sidebar is the main navigation: Today, Calendar, each profile's Inbox and projects, and Settings at the bottom. Counts show open tasks, with overdue tasks in red. Drag a project to reorder it within its profile. The collapse button turns the sidebar into an icon rail; Planora remembers the choice.

## Data and backups

Everything is stored in `%APPDATA%\Planora\`:

| File | Content |
|---|---|
| `data.json` | Projects and tasks (versioned schema, migrated on load) |
| `config.json` | Settings, window position, last view and filter |
| `state.json` | The last moment reminders were checked |
| `backups\data-YYYY-MM-DD.json` | One automatic backup per day, the last 14 are kept |
| `backups\before-*.json` | A safety copy made before an import or restore |

Writes are atomic (temp file, then rename). If `data.json` cannot be read, it is renamed to `data.corrupt-<timestamp>.json` and never overwritten; Planora starts empty and tells you where the backups are.

To test without touching your real data, set `PLANORA_DATA_DIR` to a folder. `tools/demo-data.py <folder>` writes a demo data set into it:

```powershell
python tools/demo-data.py $env:TEMP\planora-demo
$env:PLANORA_DATA_DIR = "$env:TEMP\planora-demo"
.\Planora.exe
```

With `PLANORA_DATA_DIR` set, the notification shortcut is named `Planora (test).lnk` so a real Start-menu shortcut is never touched.

## Notifications

Windows shows toasts only for an app identity that belongs to a Start-menu shortcut. Planora adds the identity `Planora.App` to `Start Menu\Programs\Planora.lnk` (it creates the shortcut if the launcher has not, and leaves an existing one untouched apart from the identity). If that fails, notifications go out under the Windows PowerShell identity instead. Notifications use WinRT toasts, not tray balloons.

## Languages

English (default), Russian and Ukrainian. The starting language follows the NIMVORO rule: `language` from the app's own config, otherwise the launcher marker `.nimvoro.json` next to the exe, otherwise English. Change it any time in Settings. Month and weekday names come from the app's own tables. The week starts on Monday (Sunday is an option for English).

## Settings

Language, theme (system, light, dark), first day of the week and time format (English), start with Windows (reflects the real registry state), close to tray, notifications, default reminder, profile colours, data tools (open folder, export, import, restore), update check.

## Build

Rust 2021, eframe/egui 0.37.

```powershell
cargo build --release   # target\release\Planora.exe
cargo test
```

Locally the GNU toolchain is used; CI (`.github/workflows/release.yml`) builds with MSVC when a `vX.Y.Z` tag is pushed and publishes `Planora-win-x64.zip`.

Other tools: `tools/make_icon.py` redraws the icon, `tools/subset_fonts.py` re-subsets the bundled font.

## Licence notes

The PDF export embeds a Latin and Cyrillic subset of Noto Sans, licensed under the SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).

## Support

If you find this app useful, you can support its development: <https://donatello.to/MicsGame>

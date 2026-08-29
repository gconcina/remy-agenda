# Remy

A notebook-style agenda app for  any Linux
environment with GTK4 

![stack](https://img.shields.io/badge/Rust-1.75%2B-orange) ![gui](https://img.shields.io/badge/UI-GTK4%20%2B%20libadwaita-blue)

## Features

- 📝 Notes with title, free-form content and an interactive checklist
- ✅ Checklist: add / toggle / delete items (saved instantly on every change)
- 🔍 Filters: All · Pending · Completed · With Reminder
- ⏰ **Recurring reminders** per note (every 1 min … daily) that reschedule
  themselves automatically and survive app restarts
- 🔔 Native desktop notifications with sound
- 📌 Tray icon (**StatusNotifier**) in the COSMIC panel — rendered as a white
  light-bulb bitmap generated in code (independent of your icon theme):
  - Left click → restore the minimized window
  - Menu: *Open Remy* / *Quit*
  - Closing with **X hides the app to the tray** instead of quitting
    (first-time hint notification included)
- 🪟 Native window decorations (minimize / maximize / close)
- 💾 JSON persistence with **atomic writes** (crash-safe tmp+rename) plus
  automatic recovery from backup if the file ever gets corrupted
- 🧪 Logic test suite (persistence round-trip, recurring reminder logic,
  corrupted-JSON tolerance)

---

## Prerequisites (system packages)

### Debian / Ubuntu / Pop!_OS 22.04+

```bash
sudo apt update
sudo apt install -y \
    build-essential pkg-config curl \
    libgtk-4-dev \
    libadwaita-1-dev
```

> `libgtk-4-dev` pulls in `libglib2.0-dev`, `gio` and the rest of the
> GTK/GLib build dependencies via `pkg-config`.

### Fedora 38+

```bash
sudo dnf install -y \
    gcc gcc-c++ make pkgconf-pkg-config curl \
    gtk4-devel \
    libadwaita-devel
```

### Arch / Manjaro

```bash
sudo pacman -S --needed \
    base-devel pkgconf curl \
    gtk4 \
    libadwaita
```

### Rust (all distros)

If you don't have a Rust toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version   # requires >= 1.75
```

### Runtime components (already shipped with COSMIC)

| Component | Purpose | Included in COSMIC? |
|---|---|---|
| Notification daemon (`cosmic-notifications`) | 🔔 reminders | ✅ Yes |
| **Status Area** panel applet | 📌 tray icon | Add via *Settings → Panel* |
| Wayland or X11 session | UI | ✅ Yes |

---

## Running the project

```bash
# 1) Clone / copy the project
cd remy-agenda

# 2) Build (debug, fast)
cargo build

# 3) Run
./target/debug/remy-agenda
```

### Production mode (recommended)

```bash
cargo build --release
./target/release/remy-agenda
```

### Tests

```bash
cargo test --release
```

3 tests must pass: note+checklist persistence round-trip, recurring reminder
logic, and corrupted-JSON tolerance.

---

## Installing as an application (optional)

```bash
# system-wide binary
sudo cp target/release/remy-agenda /usr/local/bin/

# launcher entry
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/com.github.gconcina.RemyAgenda.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Remy
Comment=Agenda with checklist and reminders
Exec=remy-agenda
Icon=text-editor-symbolic
Terminal=false
Categories=Office;Utility;
X-COSMIC-Application=true
EOF

update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
```

The app now shows up in the COSMIC launcher (Super key).

---

## Where your data lives

| File | Contents |
|---|---|
| `~/.local/share/remy-agenda/data.json` | Notes, checklists, reminders |
| `~/.local/share/remy-agenda/data.json.backup` | Automatic copy taken before every save |

Saving is **atomic** (writes `.tmp` then renames): a power cut mid-write can
never corrupt the final file. If `data.json` somehow gets corrupted, the app
tries to recover from the backup automatically.

---

## Quick start

1. **Nueva Nota / New Note** (sidebar) → type a title and content
2. Add items to the **Checklist** and tick them off
3. Press the **⏰ Repetir / Repeat** button in the editor → pick an interval
   (e.g. *Every 5 minutes*) → you'll get periodic notifications until disabled
4. **Minimize or close with X** → the 💡 tray icon stays in the panel;
   left-click it to come back
5. Real quit: the **Salir / Quit** button (sidebar) or the tray-icon menu

---

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `vkAcquireNextImageKHR ... VK_SUBOPTIMAL_KHR` on startup | Harmless notice from GTK4's Vulkan renderer. Force OpenGL:<br>`GSK_RENDERER=gl ./target/release/remy-agenda` |
| Tray icon doesn't show in the COSMIC panel | Add the **Status Area** applet: *Settings → Panel → Add applet*. The app logs `[agenda] bandeja no disponible` when no StatusNotifier host is found |
| Reminder notifications never arrive | Check the notification daemon (native in COSMIC). Sanity test: `notify-send "test"` |
| Compile error `pkg-config ... gtk4` | Missing `libgtk-4-dev` (or `gtk4-devel`) — see *Prerequisites* |
| `error: linker 'cc' not found` | Install `build-essential` (Debian/Ubuntu) or `gcc` (Fedora/Arch) |
| Window "disappeared" after closing | Look for the 💡 tray icon in the panel; left-click restores |

---

## Code structure

```
src/
├── main.rs              # GTK entry point + logic tests
├── model.rs             # Nota, ItemChecklist, AppState, FiltroNotas
├── persistence.rs       # atomic JSON + backup + recovery
├── notifications.rs     # notify-rust (desktop notifications + sound)
├── tray.rs              # StatusNotifier icon (ksni) + thread bridge
└── ui/
    ├── main_window.rs   # AdwApplicationWindow, timers, hide-to-tray
    ├── sidebar.rs       # filters, notes list, quit button
    ├── note_editor.rs   # editor + checklist + recurring reminder picker
    └── overlay.rs       # (reserved for future in-app overlay)
```

Stack: **Rust + GTK4 + libadwaita** (official gtk4-rs bindings),
`notify-rust`, `ksni`, `serde`/`chrono`/`uuid`.

## License

MPL-2.0

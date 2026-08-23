# Mi Agenda

Agenda tipo libreta para el escritorio **COSMIC** (y cualquier entorno Linux con
GTK4): notas con **checklists**, **recordatorios periódicos** con notificación
del sistema, **ícono en la bandeja del panel** y persistencia automática.

![stack](https://img.shields.io/badge/Rust-1.75%2B-orange) ![gui](https://img.shields.io/badge/UI-GTK4%20%2B%20libadwaita-blue)

## ✨ Características

- 📝 Notas con título, contenido libre y checklist interactivo
- ✅ Checklist: agregar / marcar / eliminar ítems (guardado al instante)
- 🔍 Filtros: Todas · Pendientes · Completadas · Con Recordatorio
- ⏰ Recordatorio **periódico** por nota (cada 1 min … cada día) que se
  reprograma solo; sobrevive cierres de la app
- 🔔 Notificaciones nativas del escritorio
- 📌 Ícono de bandeja (**StatusNotifier**) en el panel COSMIC:
  - Clic izquierdo → restaura la ventana minimizada
  - Menú *Abrir* / *Salir*
  - La **X** oculta a la bandeja en vez de cerrar (con aviso único)
- 💾 Persistencia JSON con **escritura atómica** (a prueba de cortes) y
  recuperación automática desde backup si el archivo se corrompe
- 🧪 Suite de tests de lógica (persistencia + recordatorios)

---

## 📦 Requisitos previos (paquetes del sistema)

### Debian / Ubuntu / Pop!_OS 22.04+

```bash
sudo apt update
sudo apt install -y \
    build-essential pkg-config curl \
    libgtk-4-dev \
    libadwaita-1-dev
```

> `libgtk-4-dev` arrastra `libglib2.0-dev`, `libgio*` y demás dependencias de
> compilación de GTK/GLib vía `pkg-config`.

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

### Rust (todas las distros)

Si no tenés toolchain de Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version   # se requiere ≥ 1.75
```

### En tiempo de ejecución (ya incluidos en COSMIC)

| Componente | Para qué | ¿Lo trae COSMIC? |
|---|---|---|
| Servidor de notificaciones (ej. `cosmic-notifications`) | 🔔 recordatorios | ✅ Sí |
| Applet **Área de estado** en el panel | 📌 ícono de bandeja | Instalable desde *Configuración → Panel* |
| Sesión Wayland o X11 | UI | ✅ Sí |

---

## 🚀 Ejecutar el proyecto

```bash
# 1) Clonar / copiar el proyecto
cd mi-agenda-gtk

# 2) Compilar (debug, rápido)
cargo build

# 3) Correr
./target/debug/mi-agenda-gtk
```

### Modo producción (recomendado)

```bash
cargo build --release
./target/release/mi-agenda-gtk
```

### Tests

```bash
cargo test --release
```

Deben pasar 3 tests: roundtrip nota+checklist, lógica de recordatorio
periódico, y tolerancia a JSON corrupto.

---

## 🖥️ Instalar como aplicación (opcional)

```bash
# binario al sistema
sudo cp target/release/mi-agenda-gtk /usr/local/bin/

# entrada de menú/lanzador
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/com.tuusuario.MiAgendaGTK.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Mi Agenda
Comment=Agenda con checklist y recordatorios
Exec=mi-agenda-gtk
Icon=text-editor-symbolic
Terminal=false
Categories=Office;Utility;
X-COSMIC-Application=true
EOF

update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
```

Ahora aparece en el lanzador COSMIC (tecla Super).

---

## 🗂️ Dónde viven tus datos

| Archivo | Contenido |
|---|---|
| `~/.local/share/mi-agenda-gtk/data.json` | Notas, checklists, recordatorios |
| `~/.local/share/mi-agenda-gtk/data.json.backup` | Copia automática previa a cada guardado |

El guardado es **atómico** (escribe `.tmp` + renombra): un corte de luz a
mitad nunca corrompe el archivo final. Si `data.json` llegara a corromperse,
la app intenta recuperar desde `.backup` sola.

---

## 🧭 Uso rápido

1. **Nueva Nota** (barra lateral) → escribe título y contenido
2. Agregá ítems al **Checklist** y marcalos
3. Botón **⏰ Repetir** en el editor → elegí intervalo (ej. *Cada 5 minutos*)
   → recibirás notificaciones periódicas hasta desactivarlo
4. **Minimizás o cerrás con X** → seguís viendo el ícono 📝 en el panel;
   clic izquierdo para volver
5. Salir real: botón **Salir** (sidebar) o menú del ícono de bandeja

---

## 🛠️ Solución de problemas

| Síntoma | Causa / solución |
|---|---|
| `vkAcquireNextImageKHR ... VK_SUBOPTIMAL_KHR` al iniciar | Aviso inocuo del renderer Vulkan de GTK4. Forzá OpenGL:<br>`GSK_RENDERER=gl ./target/release/mi-agenda-gtk` |
| No veo el ícono en el panel COSMIC | Agregá el applet **Área de estado**: *Configuración → Panel → Agregar applet*. La app loguea `[agenda] bandeja no disponible` si no hay host StatusNotifier |
| No llegan notificaciones de recordatorios | Verificá el daemon de notificaciones (en COSMIC es nativo). Probá: `notify-send "test"` |
| Error de compilación `pkg-config ... gtk4` | Falta `libgtk-4-dev` (o `gtk4-devel`) — sección *Requisitos previos* |
| `error: linker 'cc' not found` | Falta `build-essential` (Debian/Ubuntu) o `gcc` (Fedora/Arch) |
| La ventana no aparece tras ocultar | Buscá el ícono 📝 en el panel; clic izquierdo restaura |

---

## 🏗️ Estructura del código

```
src/
├── main.rs              # entry point GTK + tests de lógica
├── model.rs             # Nota, ItemChecklist, AppState, FiltroNotas
├── persistence.rs       # JSON atómico + backup + recuperación
├── notifications.rs     # notify-rust (notificaciones de escritorio)
├── tray.rs              # ícono StatusNotifier (ksni) + puente de hilos
└── ui/
    ├── main_window.rs   # AdwApplicationWindow, timers, cierre a bandeja
    ├── sidebar.rs       # filtros, lista de notas, botón Salir
    ├── note_editor.rs   # editor + checklist + recordatorio periódico
    └── overlay.rs       # (reservado para overlay in-app)
```

Stack: **Rust + GTK4 + libadwaita** (bindings oficiales gtk4-rs),
`notify-rust`, `ksni`, `serde`/`chrono`/`uuid`.

## 📄 Licencia

MPL-2.0

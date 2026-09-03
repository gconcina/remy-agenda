//! Sistema de internacionalización minimalista (ES / EN).
//!
//! Strings centralizados como pares (clave → texto_es, texto_en).
//! Se accede vía `t(idioma, clave)` o `TEXTO[idioma][clave]`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Idiomas soportados. Se persiste en `AppState.idioma`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Idioma {
    #[default]
    Espanol,
    Ingles,
}

/// Idioma activo del proceso. Inmutable salvo a través de `set_idioma()`.
/// Se sincroniza con `AppState.idioma` al arrancar y al cambiar desde
/// la ventana de Opciones.
static IDIOMA_ACTUAL: Mutex<Idioma> = Mutex::new(Idioma::Espanol);

/// Devuelve el idioma actualmente activo.
pub fn idioma_actual() -> Idioma {
    *IDIOMA_ACTUAL.lock().expect("poisoned IDIOMA_ACTUAL")
}

/// Cambia el idioma activo. Llamar después de persistirlo en `AppState`.
pub fn set_idioma(n: Idioma) {
    *IDIOMA_ACTUAL.lock().expect("poisoned IDIOMA_ACTUAL") = n;
}

impl Idioma {
    pub fn codigo(self) -> &'static str {
        match self {
            Idioma::Espanol => "es",
            Idioma::Ingles => "en",
        }
    }
}

/// Banco de strings: clave → (es, en).
type Banco = &'static [(&'static str, &'static str, &'static str)];

const ES: &[(&str, &str, &str)] = &[
    // --- Sidebar ---
    ("app.titulo_sidebar",       "Agenda",                       "Agenda"),
    ("app.nueva_nota",           "Nueva Nota",                   "New Note"),
    ("app.filtros",              "FILTROS",                      "FILTERS"),
    ("app.notas",                "NOTAS",                        "NOTES"),
    ("app.sin_notas",            "(sin notas)",                  "(no notes)"),
    ("app.sin_titulo",           "(sin título)",                 "(untitled)"),
    ("app.estado_completada",    "completada",                   "completed"),
    ("app.estado_pendiente",     "pendiente",                    "pending"),
    ("app.salir",                "Salir",                        "Quit"),
    ("app.iniciar_minimizado",   "Iniciar minimizado",           "Start minimized"),
    ("app.iniciar_minimizado_sub","Arranca oculta en la bandeja del panel", "Starts hidden in the panel tray"),
    ("app.idioma",               "Idioma",                       "Language"),
    ("app.opciones",             "Opciones",                     "Preferences"),
    // --- Welcome / editor vacío ---
    ("editor.bienvenida_titulo", "Bienvenido a Remy",            "Welcome to Remy"),
    ("editor.bienvenida_sub",    "Empezá creando tu primera nota para comenzar", "Start by creating your first note"),
    ("editor.sin_nota_sel",      "Sin nota seleccionada.\nUsá \"Nueva Nota\" en la barra lateral.", "No note selected.\nUse \"New Note\" in the sidebar."),
    // --- Editor header ---
    ("editor.titulo_ph",         "Título de la nota",            "Note title"),
    // --- Filtros ---
    ("filtro.todas",             "Todas",                        "All"),
    ("filtro.pendientes",        "Pendientes",                   "Pending"),
    ("filtro.completadas",       "Completadas",                  "Completed"),
    ("filtro.con_recordatorio",  "Con Recordatorio",             "With reminder"),
    // --- Repetir (intervalo) ---
    ("rep.etiqueta",             "Repetir",                      "Repeat"),
    ("rep.desactivar",           "Desactivar",                   "Off"),
    ("rep.cada_1m",              "Cada 1 minuto",                "Every 1 minute"),
    ("rep.cada_5m",              "Cada 5 minutos",               "Every 5 minutes"),
    ("rep.cada_15m",             "Cada 15 minutos",              "Every 15 minutes"),
    ("rep.cada_30m",             "Cada 30 minutos",              "Every 30 minutes"),
    ("rep.cada_1h",              "Cada 1 hora",                  "Every 1 hour"),
    ("rep.cada_dia",             "Cada día",                     "Every day"),
    // --- Recordatorio (one-shot) ---
    ("rec.etiqueta",             "Recordatorio",                 "Reminder"),
    ("rec.en_5m",                "En 5 min",                     "In 5 min"),
    ("rec.en_15m",               "En 15 min",                    "In 15 min"),
    ("rec.en_1h",                "En 1 h",                       "In 1 h"),
    ("rec.hora",                 "Hora:",                        "Hour:"),
    ("rec.min",                  "Min:",                         "Min:"),
    ("rec.aplicar",              "Aplicar",                      "Apply"),
    ("rec.desactivar",           "Desactivar",                   "Off"),
    ("rec.mensaje_label",        "Mensaje del recordatorio:",    "Reminder message:"),
    ("rec.mensaje_ph",           "Texto que se verá en la notificación (opcional)", "Text shown in the notification (optional)"),
    // --- Semanal ---
    ("sem.label",                "Repetir semanalmente:",        "Repeat weekly:"),
    ("sem.limpiar",              "Limpiar",                      "Clear"),
    // --- Diálogos ---
    ("dlg.fecha_pasada_titulo",  "La fecha y hora seleccionadas ya pasaron.", "The selected date and time have already passed."),
    ("dlg.fecha_pasada_sub",     "No se va a establecer el recordatorio porque es anterior al momento actual.", "The reminder won't be set because it's earlier than the current moment."),
    ("dlg.aceptar",              "Aceptar",                      "OK"),
    // --- Checklist ---
    ("checklist.titulo",         "Checklist",                    "Checklist"),
    ("checklist.agregar_ph",     "Agregar item al checklist...", "Add checklist item..."),
    ("checklist.agregar_btn",    "Agregar",                      "Add"),
    // --- Ventana de Preferencias ---
    ("pref.titulo",              "Opciones",                     "Preferences"),
    ("pref.seccion_general",     "General",                      "General"),
    ("pref.iniciar_minimizado",  "Iniciar minimizado",           "Start minimized"),
    ("pref.iniciar_minimizado_sub","Arranca oculta en la bandeja del panel", "Starts hidden in the panel tray"),
    ("pref.idioma",              "Idioma",                       "Language"),
    ("pref.idioma_sub",          "Cambiar el idioma reinicia los textos visibles.", "Changing the language refreshes the visible text."),
    ("pref.cerrar",              "Cerrar",                       "Close"),
    ("pref.renicio_parcial",     "Los cambios de idioma se aplican de inmediato.", "Language changes apply immediately."),
    // --- Tray (bandeja del panel) ---
    ("tray.appname",             "Remy",                         "Remy"),
    ("tray.title",               "Remy",                         "Remy"),
    ("tray.tooltip_desc",        "Agenda con checklist y recordatorios", "Notes, checklists and reminders"),
    ("tray.abrir",               "Abrir Remy",                   "Open Remy"),
    ("tray.salir",               "Salir",                        "Quit"),
    // --- Notificaciones ---
    ("notif.inicializado",       "Sistema de notificaciones inicializado", "Notification system initialized"),
    ("notif.recordatorio_periodico","Recordatorio periódico: {titulo}", "Periodic reminder: {titulo}"),
    ("notif.recordatorio",       "Recordatorio: {titulo}",       "Reminder: {titulo}"),
    ("notif.recordatorio_semanal","Recordatorio semanal: {titulo}","Weekly reminder: {titulo}"),
    ("notif.minimizado_titulo",  "Remy sigue abierto",           "Remy is still running"),
    ("notif.minimizado_cuerpo",  "Se ocultó al área de estado del panel: usá el ícono para restaurarla o elegir Salir.", "It was hidden to the status area: click the icon to restore it or choose Quit."),
];

/// Construye el mapa de strings para el idioma dado.
pub fn mapa(idioma: Idioma) -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    for (k, es, en) in ES {
        m.insert(*k, match idioma {
            Idioma::Espanol => *es,
            Idioma::Ingles => *en,
        });
    }
    m
}

/// Atajo: devuelve el string para el idioma y clave dados.
/// Si la clave no existe, devuelve la clave misma (visible en debug).
pub fn t(idioma: Idioma, clave: &str) -> String {
    mapa(idioma)
        .get(clave)
        .copied()
        .unwrap_or(clave)
        .to_string()
}

/// Como `t` pero reemplaza el placeholder `{titulo}` por el string dado.
/// Útil para notificaciones con formato "Recordatorio: {titulo}".
pub fn t_fmt(idioma: Idioma, clave: &str, titulo: &str) -> String {
    t(idioma, clave).replace("{titulo}", titulo)
}

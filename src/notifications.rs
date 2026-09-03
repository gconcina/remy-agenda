use crate::i18n;
use crate::model::NotificacionPendiente;
use notify_rust::{Notification, Timeout};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;
use uuid::Uuid;

lazy_static::lazy_static! {
    static ref NOTIFICACIONES_ACTIVAS: Arc<Mutex<HashMap<Uuid, notify_rust::NotificationHandle>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub fn inicializar() -> Result<(), notify_rust::error::Error> {
    let idioma = i18n::idioma_actual();
    Notification::new()
        .appname(&i18n::t(idioma, "tray.appname"))
        .summary(&i18n::t(idioma, "tray.appname"))
        .body(&i18n::t(idioma, "notif.inicializado"))
        .timeout(Timeout::Milliseconds(2000))
        .show()?;
    Ok(())
}

pub fn mostrar_notificacion(notif: &NotificacionPendiente) -> Result<(), notify_rust::error::Error> {
    cerrar_todas_notificaciones();

    let handle = Notification::new()
        .appname(&i18n::t(i18n::idioma_actual(), "tray.appname"))
        .summary(&notif.titulo)
        .body(&notif.mensaje)
        .icon("preferences-system-time")
        .timeout(Timeout::Milliseconds(30_000))
        .sound_name("message-new-instant")
        .show()?;

    std::thread::spawn(|| {
        let _ = std::process::Command::new("paplay")
            .arg("/usr/share/sounds/freedesktop/stereo/message-new-instant.oga")
            .output();
    });

    let mut activas = NOTIFICACIONES_ACTIVAS.lock().unwrap();
    activas.insert(notif.nota_id, handle);

    info!("Notificación mostrada para nota {}", notif.nota_id);
    Ok(())
}

pub fn cerrar_notificacion(nota_id: uuid::Uuid) {
    let mut activas = NOTIFICACIONES_ACTIVAS.lock().unwrap();
    if let Some(handle) = activas.remove(&nota_id) {
        handle.close();
        info!("Notificación cerrada para nota {}", nota_id);
    }
}

pub fn cerrar_todas_notificaciones() {
    let mut activas = NOTIFICACIONES_ACTIVAS.lock().unwrap();
    for (id, handle) in activas.drain() {
        handle.close();
        info!("Notificación cerrada al salir para nota {}", id);
    }
}

pub fn manejar_accion(_nota_id: uuid::Uuid, accion: &str) -> Option<crate::ui::OverlayAccion> {
    match accion {
        "abrir" => Some(crate::ui::OverlayAccion::AbrirNota),
        "posponer" => Some(crate::ui::OverlayAccion::Posponer10Min),
        "descartar" => Some(crate::ui::OverlayAccion::Descartar),
        _ => None,
    }
}
// Ícono de bandeja del sistema vía StatusNotifier (protocolo que el
// área de estado del panel COSMIC implementa). Crate ksni: Rust puro.
//
// Puente de hilos: los callbacks de ksni corren en SU hilo dbus. Los objetos
// GTK no son Send, así que guardamos el puntero de un clon con vida útil de
// la aplicación (leaked) y lo usamos SOLO desde MainContext::invoke,
// que ejecuta en el hilo principal de GTK.
use crate::model::AppState;
use gtk4::prelude::*;
use ksni::menu::{MenuItem, StandardItem};
use ksni::Tray;
use std::sync::{Arc, Mutex};

static WIN_PTR: Mutex<Option<usize>> = Mutex::new(None);
static APP_STATE: Mutex<Option<Arc<Mutex<AppState>>>> = Mutex::new(None);

/// Registra ventana y estado; lanza el servicio de bandeja en su propio hilo.
pub fn iniciar(win: &libadwaita::ApplicationWindow, state: Arc<Mutex<AppState>>) {
    // Clon fuerte filtrado a propósito: vive mientras viva la app.
    let owned = win.clone();
    {
        *WIN_PTR.lock().unwrap() = Some(Box::into_raw(Box::new(owned)) as usize);
        *APP_STATE.lock().unwrap() = Some(state);
    }

    std::thread::spawn(move || {
        let service = ksni::TrayService::new(AgendaTray);
        if let Err(e) = service.run() {
            eprintln!(
                "[agenda] bandeja no disponible (¿falta 'Área de estado' en el panel?): {e}"
            );
        }
    });

    println!("[agenda] bandeja iniciando...");
}

/// Ejecuta un closure en el hilo principal de GTK (thread-safe)
fn en_hilo_principal<F: FnOnce() + Send + 'static>(f: F) {
    glib::MainContext::default().invoke(f);
}

/// Obtiene la ventana registrada. SEGURA solo en el hilo principal.
fn ventana_en_hilo_principal() {
    let ptr = WIN_PTR.lock().unwrap().unwrap();
    // SAFETY: puntero válido toda la vida de la app; estamos en hilo principal.
    let win = unsafe { &*(ptr as *const libadwaita::ApplicationWindow) };
    win.present(); // des-minimiza y trae al frente
    println!("[agenda] ventana restaurada desde bandeja");
}

fn mostrar_ventana() {
    en_hilo_principal(ventana_en_hilo_principal);
}

fn salir_app() {
    en_hilo_principal(|| {
        println!("[agenda] salir desde bandeja");
        if let Some(st) = &*APP_STATE.lock().unwrap() {
            if let Ok(s) = st.lock() {
                let _ = crate::persistence::guardar_datos(&s);
            }
        }
        crate::notifications::cerrar_todas_notificaciones();
        if let Some(app) = gio::Application::default() {
            app.quit();
        }
    });
}

pub struct AgendaTray;

impl Tray for AgendaTray {
    fn id(&self) -> String {
        "mi-agenda-gtk".into()
    }

    fn icon_name(&self) -> String {
        "text-editor-symbolic".into()
    }

    fn title(&self) -> String {
        "Mi Agenda".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: "text-editor-symbolic".into(),
            title: "Mi Agenda".into(),
            description: "Agenda con checklist y recordatorios".into(),
            ..Default::default()
        }
    }

    /// Clic izquierdo sobre el ícono
    fn activate(&mut self, _x: i32, _y: i32) {
        mostrar_ventana();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            MenuItem::Standard(StandardItem {
                label: "Abrir Mi Agenda".into(),
                activate: Box::new(|_| mostrar_ventana()),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Salir".into(),
                activate: Box::new(|_| salir_app()),
                ..Default::default()
            }),
        ]
    }
}

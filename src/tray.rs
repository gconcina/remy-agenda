// Ícono de bandeja del sistema vía StatusNotifier (protocolo que el
// área de estado del panel COSMIC implementa). Crate ksni: Rust puro.
//
// Puente de hilos: los callbacks de ksni corren en SU hilo dbus. Los objetos
// GTK no son Send, así que guardamos el puntero de un clon con vida útil de
// la aplicación (leaked) y lo usamos SOLO desde MainContext::invoke,
// que ejecuta en el hilo principal de GTK.
use crate::i18n;
use crate::model::AppState;
use gtk4::prelude::*;
use ksni::menu::{MenuItem, StandardItem};
use ksni::Tray;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

static WIN_PTR: Mutex<Option<usize>> = Mutex::new(None);
static APP_STATE: Mutex<Option<Arc<Mutex<AppState>>>> = Mutex::new(None);
static TRAY_INICIALIZADO: AtomicBool = AtomicBool::new(false);

/// Registra ventana y estado; lanza el servicio de bandeja en su propio hilo.
///
/// Idempotente: si ya se inicializó (típicamente porque `MainWindow::new` se
/// llamó otra vez tras un cambio de idioma), no vuelve a registrar el tray
/// ni a crear un nuevo hilo dbus — sólo actualiza el puntero a la ventana
/// actual (liberando el anterior para evitar leaks).
pub fn iniciar(win: &libadwaita::ApplicationWindow, state: Arc<Mutex<AppState>>) {
    let new_ptr = Box::into_raw(Box::new(win.clone())) as usize;
    {
        let mut win_slot = WIN_PTR.lock().unwrap();
        if let Some(old) = *win_slot {
            // Liberar la referencia anterior (no usar desde ya nadie).
            unsafe { drop(Box::from_raw(old as *mut libadwaita::ApplicationWindow)); }
        }
        *win_slot = Some(new_ptr);
        *APP_STATE.lock().unwrap() = Some(state);
    }

    if TRAY_INICIALIZADO.swap(true, Ordering::SeqCst) {
        return;
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
        "remy-agenda".into()
    }

    fn icon_name(&self) -> String {
        String::new()
    }

    /// Bombilla 24×24 generada por código (ARGB32): no depende del tema
    /// de íconos del sistema, que puede no tener ninguna.
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        const S: i32 = 24;
        let mut data = Vec::with_capacity((S * S * 4) as usize);

        for y in 0..S {
            for x in 0..S {
                let dx = x - 12;
                let dy = y - 10;
                let d2 = dx * dx + dy * dy;

                let (a, r, g, b);
                if d2 <= 49 {
                    let hx = x - 9;
                    let hy = y - 7;
                    if hx * hx + hy * hy <= 4 {
                        (a, r, g, b) = (255, 255, 255, 255);
                    } else if ((x == 10 || x == 14) && (11..=14).contains(&y))
                        || (y == 11 && (10..=14).contains(&x))
                    {
                        (a, r, g, b) = (255, 165, 165, 170);
                    } else {
                        (a, r, g, b) = (255, 235, 235, 235);
                    }
                } else if (y == 18 && (9..=15).contains(&x))
                    || (y == 19 && (9..=15).contains(&x))
                    || (y == 20 && (10..=14).contains(&x))
                    || (y == 21 && (11..=13).contains(&x))
                {
                    (a, r, g, b) = (255, 185, 185, 190);
                } else {
                    (a, r, g, b) = (0, 0, 0, 0);
                }

                data.push(a);
                data.push(r);
                data.push(g);
                data.push(b);
            }
        }

        vec![ksni::Icon {
            width: S,
            height: S,
            data,
        }]
    }

    fn title(&self) -> String {
        i18n::t(i18n::idioma_actual(), "tray.title")
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: String::new(),
            title: i18n::t(i18n::idioma_actual(), "tray.title"),
            description: i18n::t(i18n::idioma_actual(), "tray.tooltip_desc"),
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
                label: i18n::t(i18n::idioma_actual(), "tray.abrir"),
                activate: Box::new(|_| mostrar_ventana()),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: i18n::t(i18n::idioma_actual(), "tray.salir"),
                activate: Box::new(|_| salir_app()),
                ..Default::default()
            }),
        ]
    }
}

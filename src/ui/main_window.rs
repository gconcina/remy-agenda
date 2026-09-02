use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box as GtkBox, Orientation};
use libadwaita::{ViewStack, ApplicationWindow as AdwApplicationWindow};
use std::sync::{Arc, Mutex};
use crate::model::AppState;
use crate::persistence::cargar_datos;
use crate::notifications::{inicializar as init_notifications, cerrar_todas_notificaciones};
use crate::ui::{sidebar, note_editor};

pub struct MainWindow {
    window: AdwApplicationWindow,
    state: Arc<Mutex<AppState>>,
    #[allow(dead_code)]
    view_stack: ViewStack,
}

impl MainWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        if let Err(e) = init_notifications() {
            eprintln!("[agenda] notificaciones no disponibles: {e}");
        }

        let state = Arc::new(Mutex::new(cargar_datos().unwrap_or_default()));

        // Páginas del stack
        let view_stack = ViewStack::new();

        let welcome_page = note_editor::create_welcome_page();
        view_stack.add_titled(&welcome_page, Some("welcome"), "Bienvenida");

        // El editor es un contenedor vacío cuyo contenido gestiona note_editor
        let editor_page = GtkBox::new(Orientation::Vertical, 0);
        editor_page.set_hexpand(true);
        editor_page.set_vexpand(true);
        view_stack.add_titled(&editor_page, Some("editor"), "Editor");

        // Registrar el área de contenido del editor (thread_local)
        note_editor::init_editor(editor_page);

        // Estado inicial: mostrar bienvenida o primera nota si hay datos
        {
            let has_current = state.lock().unwrap().nota_actual.is_some();
            if has_current {
                note_editor::update_editor_for_note(Arc::clone(&state));
                view_stack.set_visible_child_name("editor");
            } else {
                view_stack.set_visible_child_name("welcome");
            }
        }

        // Sidebar
        let sidebar_widget = sidebar::create_sidebar(Arc::clone(&state), view_stack.clone());
        sidebar::append_salir_button(&sidebar_widget, Arc::clone(&state));

        // Layout principal
        let main_box = GtkBox::new(Orientation::Horizontal, 0);
        main_box.append(&sidebar_widget);
        main_box.append(&view_stack);

        // Decoraciones de ventana: barra de título con minimizar/maximizar/cerrar
        let header_bar = libadwaita::HeaderBar::new();
        let outer = GtkBox::new(Orientation::Vertical, 0);
        outer.append(&header_bar);
        outer.append(&main_box);

        let window = AdwApplicationWindow::builder()
            .application(app)
            .title("Remy")
            .default_width(1000)
            .default_height(700)
            .content(&outer)
            .build();

        // La X OCULTA a la bandeja (guardando); salir solo con botones Salir
        {
            let st = Arc::clone(&state);
            let win_hide = window.clone();
            window.connect_close_request(move |_| {
                println!("[agenda] X presionada: ocultando a bandeja...");
                if let Ok(s) = st.lock() {
                    let _ = crate::persistence::guardar_datos(&s);
                }
                win_hide.hide();

                // Aviso una única vez para que no parezca que se cerró
                static AVISO_DADO: Mutex<bool> = Mutex::new(false);
                let mut aviso = AVISO_DADO.lock().unwrap();
                if !*aviso {
                    *aviso = true;
                    drop(aviso);
                    std::thread::spawn(|| {
                        let _ = notify_rust::Notification::new()
                            .appname("Remy")
                            .summary("Remy sigue abierta")
                            .body("Se ocultó al área de estado del panel: usá el ícono para restaurarla o elegir Salir.")
                            .timeout(notify_rust::Timeout::Milliseconds(5000))
                            .show();
                    });
                }

                glib::Propagation::Stop // NO destruye la ventana
            });
        }

        // Bandeja del sistema (ícono en el panel COSMIC)
        crate::tray::iniciar(&window, Arc::clone(&state));

        let win = Self {
            window,
            state,
            view_stack,
        };

        win.start_auto_save();
        win.start_reminder_check();
        win
    }

    fn start_auto_save(&self) {
        let st = Arc::clone(&self.state);
        glib::timeout_add_seconds_local(
            60,
            move || {
                if let Ok(s) = st.lock() {
                    let _ = crate::persistence::guardar_datos(&s);
                }
                glib::ControlFlow::Continue
            },
        );
    }

    fn start_reminder_check(&self) {
        let st = Arc::clone(&self.state);
        // Chequeo cada 10 segundos para soportar intervalos de 1 minuto
        glib::timeout_add_seconds_local(
            10,
            move || {
                let mut s = match st.lock() {
                    Ok(s) => s,
                    Err(_) => return glib::ControlFlow::Continue,
                };
                let ahora = chrono::Local::now();
                let ids: Vec<uuid::Uuid> = s.notas.keys().cloned().collect();

                // Recolectar recordatorios vencidos
                let mut disparos: Vec<(uuid::Uuid, String, String)> = Vec::new();
                for id in ids {
                    let Some(nota) = s.notas.get_mut(&id) else { continue };

                    // 1. Recordatorio periódico
                    if let Some(intervalo) = nota.intervalo_segundos {
                        let delta = chrono::Duration::seconds(intervalo as i64);
                        match nota.proximo_recordatorio {
                            None => {
                                // Primera vez: programa desde ahora
                                nota.proximo_recordatorio = Some(ahora + delta);
                            }
                            Some(proximo) if proximo <= ahora => {
                                let titulo_nota = if nota.titulo.is_empty() {
                                    "(sin título)".to_string()
                                } else {
                                    nota.titulo.clone()
                                };
                                let cuerpo = nota
                                    .recordatorio_mensaje
                                    .as_deref()
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                    .map(str::to_string)
                                    .unwrap_or_else(|| format!("Recordatorio periódico: {}", nota.titulo));
                                disparos.push((id, titulo_nota, cuerpo));
                                // Reprograma SIEMPRE, aunque la app estuviera cerrada
                                nota.proximo_recordatorio = Some(ahora + delta);
                            }
                            _ => {}
                        }
                    }

                    // 2. Recordatorio único (legado)
                    if let Some(rec) = nota.recordatorio {
                        if rec <= ahora {
                            let cuerpo = nota
                                .recordatorio_mensaje
                                .as_deref()
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(str::to_string)
                                .unwrap_or_else(|| format!("Recordatorio: {}", nota.titulo));
                            disparos.push((id, nota.titulo.clone(), cuerpo));
                            nota.recordatorio = None; // one-shot: se consume
                        }
                    }
                }
                drop(s);

                // Enviar notificaciones del sistema fuera del lock
                for (id, titulo, mensaje) in disparos {
                    let notif = crate::model::NotificacionPendiente {
                        nota_id: id,
                        titulo,
                        mensaje,
                        cuando: ahora,
                        mostrada: true,
                    };
                    if let Err(e) = crate::notifications::mostrar_notificacion(&notif) {
                        eprintln!("[agenda] error notificación: {e}");
                    }
                    println!("[agenda] 🔔 recordatorio disparado para nota {id}");
                }

                glib::ControlFlow::Continue
            },
        );
    }

    pub fn present(&self) {
        self.window.present();
    }

    /// Muestra la ventana, salvo que el ajuste "iniciar_minimizado" esté
    /// activo: en ese caso la crea y la oculta en la bandeja del sistema.
    pub fn present_or_hide(&self) {
        self.window.present();
        let minimizar = match self.state.lock() {
            Ok(s) => s.iniciar_minimizado,
            Err(_) => return,
        };
        if minimizar {
            self.window.hide();
            println!("[agenda] arranque minimizado: ventana oculta en la bandeja");
        }
    }
}

impl Drop for MainWindow {
    fn drop(&mut self) {
        cerrar_todas_notificaciones();
        if let Ok(s) = self.state.lock() {
            let _ = crate::persistence::guardar_datos(&s);
        }
    }
}

//! Ventana de Preferencias (AdwPreferencesWindow).
//!
//! Contiene:
//! - "Iniciar minimizado": checkbox persistido en `AppState`.
//! - "Idioma": dropdown Español / English. Al cambiar aplica en caliente
//!   llamando al callback `on_idioma_change` que provee MainWindow.

use gtk4::prelude::*;
use libadwaita::prelude::*;
use libadwaita::{PreferencesWindow as AdwPreferencesWindow, PreferencesPage, PreferencesGroup};
use gtk4::{Box as GtkBox, Orientation, CheckButton, DropDown};
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use crate::model::AppState;
use crate::i18n::{self, Idioma};

/// Callback invocado cuando el usuario cambia el idioma.
/// MainWindow lo provee para reconstruir el sidebar + editor.
/// `Rc` para poder clonarlo al capturar dentro de la closure del DropDown.
pub type OnIdiomaChange = Rc<dyn Fn()>;

/// Builder de la ventana de Preferencias. No almacena estado propio: cada
/// llamada a `present()` crea una ventana nueva (la app puede reabrirla).
#[derive(Default)]
pub struct PreferencesDialog {
    pub on_idioma_change: Option<OnIdiomaChange>,
}

impl PreferencesDialog {
    /// Construye y muestra la ventana de preferencias anclada a `parent`.
    pub fn present(self, parent: &gtk4::Window, state: Arc<Mutex<AppState>>) {
        let idioma = i18n::idioma_actual();

        let win = AdwPreferencesWindow::new();
        win.set_title(Some(&i18n::t(idioma, "pref.titulo")));
        win.set_transient_for(Some(parent));
        win.set_modal(true);
        win.set_default_size(420, 320);

        let page = PreferencesPage::new();
        page.set_title(&i18n::t(idioma, "pref.titulo"));
        page.set_icon_name(Some("preferences-system-symbolic"));

        // --- Sección General ---
        let general = PreferencesGroup::new();
        general.set_title(&i18n::t(idioma, "pref.seccion_general"));

        // Iniciar minimizado
        let min_row = libadwaita::ActionRow::new();
        min_row.set_title(&i18n::t(idioma, "pref.iniciar_minimizado"));
        min_row.set_subtitle(&i18n::t(idioma, "pref.iniciar_minimizado_sub"));
        let check_min = CheckButton::new();
        check_min.set_valign(gtk4::Align::Center);
        check_min.set_active(state.lock().map(|s| s.iniciar_minimizado).unwrap_or(false));
        min_row.add_suffix(&check_min);
        min_row.set_activatable(true);
        {
            let chk = check_min.clone();
            min_row.connect_activated(move |_| {
                chk.set_active(!chk.is_active());
            });
            let st = Arc::clone(&state);
            check_min.connect_toggled(move |c| {
                let mut s = st.lock().unwrap();
                s.iniciar_minimizado = c.is_active();
                let _ = crate::persistence::guardar_datos(&s);
            });
        }
        general.add(&min_row);

        // Idioma
        let idioma_row = libadwaita::ActionRow::new();
        idioma_row.set_title(&i18n::t(idioma, "pref.idioma"));
        idioma_row.set_subtitle(&i18n::t(idioma, "pref.idioma_sub"));
        let dropdown = DropDown::from_strings(&["Español", "English"]);
        dropdown.set_selected(match idioma {
            Idioma::Espanol => 0u32,
            Idioma::Ingles => 1u32,
        });
        dropdown.set_valign(gtk4::Align::Center);
        idioma_row.add_suffix(&dropdown);
        idioma_row.set_activatable(true);
        {
            let cb = self.on_idioma_change;
            let state_for_idioma = Arc::clone(&state);
            dropdown.connect_selected_notify(move |dd| {
                let nuevo = match dd.selected() {
                    0 => Idioma::Espanol,
                    _ => Idioma::Ingles,
                };
                if i18n::idioma_actual() == nuevo {
                    return;
                }
                // Persistir el idioma antes de cambiar el global
                {
                    let mut s = state_for_idioma.lock().unwrap();
                    s.idioma = nuevo;
                    let _ = crate::persistence::guardar_datos(&s);
                }
                i18n::set_idioma(nuevo);
                // Cerrar PreferencesWindow ANTES de que la ventana padre
                // se destruya y recree (evita popover/parent inválidos).
                if let Some(ancestor) = dd.root() {
                    if let Some(win) = ancestor.downcast_ref::<gtk4::Window>() {
                        win.close();
                    }
                }
                if let Some(cb) = cb.as_ref() {
                    cb();
                }
            });
        }
        general.add(&idioma_row);

        page.add(&general);

        // Aviso de aplicación inmediata — agregar como un PreferencesGroup
        // separado para que `page.add()` acepte el tipo correcto.
        let info_group = PreferencesGroup::new();
        let aviso = gtk4::Label::new(Some(&i18n::t(idioma, "pref.renicio_parcial")));
        aviso.add_css_class("dim-label");
        aviso.set_wrap(true);
        aviso.set_xalign(0.0);
        aviso.set_margin_top(6);
        aviso.set_margin_bottom(6);
        aviso.set_margin_start(12);
        aviso.set_margin_end(12);
        info_group.add(&aviso);
        page.add(&info_group);

        win.add(&page);
        win.present();
    }
}

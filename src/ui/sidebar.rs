use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box as GtkBox, Orientation, Label, ScrolledWindow, PolicyType};
use libadwaita::ViewStack;
use std::sync::{Arc, Mutex};
use crate::model::{AppState, FiltroNotas};
use crate::ui::note_editor;

pub fn create_sidebar(state: Arc<Mutex<AppState>>, view_stack: ViewStack) -> GtkBox {
    let sidebar = GtkBox::new(Orientation::Vertical, 0);
    sidebar.set_size_request(280, -1);
    sidebar.add_css_class("sidebar");

    // Header
    let header_box = GtkBox::new(Orientation::Horizontal, 12);
    header_box.set_margin_top(12);
    header_box.set_margin_bottom(12);
    header_box.set_margin_start(16);
    header_box.set_margin_end(16);

    let title = Label::new(Some("Agenda"));
    title.add_css_class("title-1");
    header_box.append(&title);

    sidebar.append(&header_box);

    // Botón nueva nota
    let new_note_btn = gtk4::Button::new();
    new_note_btn.set_label("Nueva Nota");
    new_note_btn.set_icon_name("list-add-symbolic");
    new_note_btn.set_hexpand(true);
    new_note_btn.set_margin_top(4);
    new_note_btn.set_margin_bottom(12);
    new_note_btn.set_margin_start(16);
    new_note_btn.set_margin_end(16);
    new_note_btn.add_css_class("suggested-action");

    {
        let st = Arc::clone(&state);
        let stack_c = view_stack.clone();
        new_note_btn.connect_clicked(move |_| {
            let id = {
                let mut s = st.lock().unwrap();
                let nota = crate::model::Nota::new(String::new(), String::new());
                let id = nota.id;
                s.notas.insert(id, nota);
                s.nota_actual = Some(id);
                id
            };
            let _ = crate::persistence::guardar_datos(&st.lock().unwrap());
            note_editor::update_editor_for_note(Arc::clone(&st));
            stack_c.set_visible_child_name("editor");
            println!("[agenda] nota creada: {id}");
        });
    }
    sidebar.append(&new_note_btn);

    // Separador
    sidebar.append(&gtk4::Separator::new(Orientation::Horizontal));

    // Etiqueta filtros
    let filters_label = Label::new(Some("FILTROS"));
    filters_label.add_css_class("caption-heading");
    filters_label.set_halign(gtk4::Align::Start);
    filters_label.set_margin_start(16);
    filters_label.set_margin_top(12);
    filters_label.set_margin_bottom(4);
    sidebar.append(&filters_label);

    // Filtros como ActionRow clicables
    for filtro in FiltroNotas::todos() {
        let row = libadwaita::ActionRow::new();
        row.set_title(filtro.label());
        row.set_icon_name(Some(filtro.icon()));
        row.set_activatable(true);

        let count_label = Label::new(Some("0"));
        count_label.add_css_class("caption");
        count_label.set_valign(gtk4::Align::Center);
        row.add_suffix(&count_label);

        {
            let st = Arc::clone(&state);
            let cl = count_label.clone();
            glib::timeout_add_seconds_local(1, move || {
                let s = st.lock().unwrap();
                let count = s.notas.values().filter(|n| filtro.aplica(n)).count();
                cl.set_text(&count.to_string());
                glib::ControlFlow::Continue
            });
        }

        {
            let st = Arc::clone(&state);
            row.connect_activated(move |_| {
                let mut s = st.lock().unwrap();
                s.filtro = filtro;
                println!("[agenda] filtro: {}", filtro.label());
            });
        }

        sidebar.append(&row);
    }

    // Separador + etiqueta de notas
    sidebar.append(&gtk4::Separator::new(Orientation::Horizontal));

    let notes_header = Label::new(Some("NOTAS"));
    notes_header.add_css_class("caption-heading");
    notes_header.set_halign(gtk4::Align::Start);
    notes_header.set_margin_start(16);
    notes_header.set_margin_top(12);
    notes_header.set_margin_bottom(4);
    sidebar.append(&notes_header);

    // Lista scrolleable de notas
    let scroll = ScrolledWindow::new();
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_vexpand(true);

    let notes_list = gtk4::ListBox::new();
    notes_list.set_selection_mode(gtk4::SelectionMode::Single);
    notes_list.set_margin_top(4);
    notes_list.set_margin_bottom(8);
    notes_list.set_margin_start(8);
    notes_list.set_margin_end(8);
    scroll.set_child(Some(&notes_list));
    sidebar.append(&scroll);

    // Handler central como respaldo (lee el id desde la propiedad "name")
    {
        let st = Arc::clone(&state);
        let vs = view_stack.clone();
        notes_list.connect_row_activated(move |_list, row| {
            let name: String = row.property::<String>("name");
            if name.is_empty() {
                return;
            }
            let Ok(id) = uuid::Uuid::parse_str(&name) else { return };
            println!("[agenda] click en fila (row_activated): {id}");
            abrir_nota(&st, &vs, id);
        });
    }

    // Refresco de lista: SOLO reconstruye cuando los datos cambian de verdad,
    // evitando destruir filas mientras el usuario hace clic.
    {
        let st = Arc::clone(&state);
        let list_c = notes_list.clone();
        let vs_c = view_stack.clone();
        let previa: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
        glib::timeout_add_seconds_local(1, move || {
            let f = calcular_firma(&st);
            if f != *previa.borrow() {
                *previa.borrow_mut() = f;
                rebuild_notes_list(&list_c, &st, &vs_c);
            }
            glib::ControlFlow::Continue
        });
    }

    // Primera construcción inmediata
    rebuild_notes_list(&notes_list, &state, &view_stack);

    sidebar
}

/// Abre una nota en el editor y cambia a la página de edición
fn abrir_nota(state: &Arc<Mutex<AppState>>, view_stack: &ViewStack, id: uuid::Uuid) {
    {
        let mut s = state.lock().unwrap();
        s.nota_actual = Some(id);
    }
    note_editor::update_editor_for_note(Arc::clone(state));
    view_stack.set_visible_child_name("editor");
    println!("[agenda] nota abierta para editar: {id}");
}

/// Firma barata del estado de la lista: cantidad + último cambio + filtro
fn calcular_firma(state: &Arc<Mutex<AppState>>) -> String {
    let s = state.lock().unwrap();
    let max_act = s
        .notas
        .values()
        .map(|n| n.actualizada.timestamp())
        .max()
        .unwrap_or(0);
    format!("{}|{}|{}", s.notas.len(), max_act, s.filtro as u8)
}

fn rebuild_notes_list(list: &gtk4::ListBox, state: &Arc<Mutex<AppState>>, view_stack: &ViewStack) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let notas: Vec<crate::model::Nota> = {
        let s = state.lock().unwrap();
        let mut v: Vec<_> = s.notas_filtradas().into_iter().cloned().collect();
        v.sort_by(|a, b| b.actualizada.cmp(&a.actualizada));
        v
    };

    if notas.is_empty() {
        let empty_row = gtk4::ListBoxRow::new();
        empty_row.set_sensitive(false);
        let lbl = Label::new(Some("(sin notas)"));
        lbl.add_css_class("dim-label");
        lbl.set_margin_top(8);
        lbl.set_margin_bottom(8);
        empty_row.set_child(Some(&lbl));
        list.append(&empty_row);
        return;
    }

    for nota in notas {
        let row = libadwaita::ActionRow::new();
        // El id vive en la propiedad "name" del widget; el handler central lo lee al activar
        row.set_property("name", nota.id.to_string());
        row.set_activatable(true);

        let titulo = if nota.titulo.is_empty() {
            "(sin título)".to_string()
        } else {
            nota.titulo.clone()
        };
        let mut subtitle = format!(
            "{} · {}",
            nota.actualizada.format("%d/%m %H:%M"),
            if nota.completada { "completada" } else { "pendiente" },
        );
        if nota.intervalo_segundos.is_some() {
            subtitle.push_str(" · ⏰");
        }
        row.set_title(&titulo);
        row.set_subtitle(&subtitle);

        let icon = gtk4::Image::from_icon_name(if nota.completada {
            "task-complete-symbolic"
        } else {
            "edit-symbolic"
        });
        row.add_prefix(&icon);

        // Handler DIRECTO por fila: la vía más confiable de activación
        {
            let st = Arc::clone(state);
            let vs = view_stack.clone();
            let id = nota.id;
            row.connect_activated(move |_row| {
                println!("[agenda] click en fila (ActionRow activated): {id}");
                abrir_nota(&st, &vs, id);
            });
        }

        list.append(&row);
    }
}

/// Añade el botón Salir al final del sidebar
pub fn append_salir_button(sidebar: &GtkBox, state: Arc<Mutex<AppState>>) {
    sidebar.append(&gtk4::Separator::new(Orientation::Horizontal));
    let salir_btn = gtk4::Button::new();
    salir_btn.set_label("Salir");
    salir_btn.set_icon_name("application-exit-symbolic");
    salir_btn.set_hexpand(true);
    salir_btn.set_margin_top(8);
    salir_btn.set_margin_bottom(12);
    salir_btn.set_margin_start(16);
    salir_btn.set_margin_end(16);
    salir_btn.add_css_class("destructive-action");
    let st = Arc::clone(&state);
    salir_btn.connect_clicked(move |_| {
        println!("[agenda] cerrando aplicación...");
        if let Ok(s) = st.lock() {
            let _ = crate::persistence::guardar_datos(&s);
        }
        crate::notifications::cerrar_todas_notificaciones();
        if let Some(app) = gio::Application::default() {
            app.quit();
        }
    });
    sidebar.append(&salir_btn);
}

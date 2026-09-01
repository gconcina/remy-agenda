use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box as GtkBox, Orientation, ScrolledWindow, PolicyType, Entry, TextView, TextBuffer, Switch, Button, Separator, Label, CheckButton};
use std::sync::{Arc, Mutex};
use crate::model::AppState;
use chrono::Timelike;
use std::cell::RefCell;

// El editor vive en el hilo principal de GTK; thread_local es la forma
// segura y simple de compartirlo sin set_data ni casts inseguros.
thread_local! {
    static CONTENT_AREA: RefCell<Option<GtkBox>> = const { RefCell::new(None) };
}

/// Registra el área de contenido del editor. Llamar una vez al iniciar.
pub fn init_editor(content_area: GtkBox) {
    CONTENT_AREA.with(|c| *c.borrow_mut() = Some(content_area));
}

fn clear_children(box_widget: &GtkBox) {
    while let Some(child) = box_widget.first_child() {
        box_widget.remove(&child);
    }
}

pub fn create_welcome_page() -> GtkBox {
    let welcome = GtkBox::new(Orientation::Vertical, 24);
    welcome.set_halign(gtk4::Align::Center);
    welcome.set_valign(gtk4::Align::Center);
    welcome.set_margin_top(64);
    welcome.set_margin_bottom(64);
    welcome.set_margin_start(32);
    welcome.set_margin_end(32);

    let icon = gtk4::Image::from_icon_name("document-new-symbolic");
    icon.set_pixel_size(64);
    welcome.append(&icon);

    let title = Label::new(Some("Bienvenido a Remy"));
    title.add_css_class("title-1");
    welcome.append(&title);

    let subtitle = Label::new(Some("Empezá creando tu primera nota para comenzar"));
    subtitle.add_css_class("body");
    subtitle.add_css_class("dim-label");
    welcome.append(&subtitle);

    welcome
}

fn show_welcome_in_editor(content_area: &GtkBox) {
    clear_children(content_area);

    let empty = GtkBox::new(Orientation::Vertical, 24);
    empty.set_halign(gtk4::Align::Center);
    empty.set_valign(gtk4::Align::Center);
    empty.set_margin_top(64);

    let icon = gtk4::Image::from_icon_name("document-new-symbolic");
    icon.set_pixel_size(64);
    empty.append(&icon);

    let msg = Label::new(Some("Sin nota seleccionada.\nUsá \"Nueva Nota\" en la barra lateral."));
    msg.add_css_class("dim-label");
    msg.set_justify(gtk4::Justification::Center);
    empty.append(&msg);

    content_area.append(&empty);
}

/// Reconstruye el editor según el estado actual (nota seleccionada o vacío).
pub fn update_editor_for_note(state: Arc<Mutex<AppState>>) {
    CONTENT_AREA.with(|cell| {
        let content_area_opt = cell.borrow();
        if let Some(content_area) = content_area_opt.as_ref() {
            let nota_opt = {
                let s = state.lock().unwrap();
                s.nota_actual.and_then(|id| s.notas.get(&id)).cloned()
            };

            match nota_opt {
                Some(nota) => build_editor_ui(content_area, &state, nota),
                None => show_welcome_in_editor(content_area),
            }
        }
    });
}

fn build_editor_ui(content_area: &GtkBox, state: &Arc<Mutex<AppState>>, nota: crate::model::Nota) {
    clear_children(content_area);
    let nota_id = nota.id;

    // ---- Header ----
    let header = GtkBox::new(Orientation::Horizontal, 12);
    header.set_margin_top(16);
    header.set_margin_bottom(12);
    header.set_margin_start(24);
    header.set_margin_end(24);

    let title_entry = Entry::new();
    title_entry.set_text(&nota.titulo);
    title_entry.set_placeholder_text(Some("Título de la nota"));
    title_entry.set_hexpand(true);
    title_entry.add_css_class("title-2");

    {
        let st = Arc::clone(state);
        title_entry.connect_changed(move |entry| {
            let mut s = st.lock().unwrap();
            if let Some(n) = s.notas.get_mut(&nota_id) {
                n.titulo = entry.text().to_string();
                n.actualizada = chrono::Local::now();
            }
            // Guardado inmediato: la edición nunca se pierde
            let _ = crate::persistence::guardar_datos(&s);
        });
    }
    header.append(&title_entry);

    let completed_switch = Switch::new();
    completed_switch.set_active(nota.completada);
    completed_switch.set_valign(gtk4::Align::Center);

    {
        let st = Arc::clone(state);
        completed_switch.connect_state_set(move |_, active| {
            let mut s = st.lock().unwrap();
            if let Some(n) = s.notas.get_mut(&nota_id) {
                n.completada = active;
                n.actualizada = chrono::Local::now();
            }
            glib::Propagation::Proceed
        });
    }
    header.append(&completed_switch);

    // Botón de recordatorio periódico (menú desplegable)
    let rem_btn = gtk4::MenuButton::new();
    let rem_label = Label::new(Some(&nota.etiqueta_intervalo()));
    let rem_box = GtkBox::new(Orientation::Horizontal, 6);
    rem_box.append(&gtk4::Image::from_icon_name("alarm-symbolic"));
    rem_box.append(&rem_label);
    rem_btn.set_child(Some(&rem_box));
    rem_btn.set_valign(gtk4::Align::Center);

    let pop = gtk4::Popover::new();
    let pop_box = GtkBox::new(Orientation::Vertical, 2);
    pop_box.set_margin_top(8);
    pop_box.set_margin_bottom(8);
    pop_box.set_margin_start(8);
    pop_box.set_margin_end(8);

    let opciones: Vec<(&str, Option<u64>)> = vec![
        ("Desactivar", None),
        ("Cada 1 minuto", Some(60)),
        ("Cada 5 minutos", Some(300)),
        ("Cada 15 minutos", Some(900)),
        ("Cada 30 minutos", Some(1800)),
        ("Cada 1 hora", Some(3600)),
        ("Cada día", Some(86400)),
    ];

    for (texto, segs) in opciones {
        let opt_btn = Button::with_label(texto);
        opt_btn.add_css_class("flat");
        {
            let st = Arc::clone(state);
            let pop_c = pop.clone();
            let lbl_c = rem_label.clone();
            opt_btn.connect_clicked(move |_| {
                let etiqueta = {
                    let mut s = st.lock().unwrap();
                    if let Some(n) = s.notas.get_mut(&nota_id) {
                        n.intervalo_segundos = segs;
                        n.proximo_recordatorio = match segs {
                            Some(iv) => Some(chrono::Local::now() + chrono::Duration::seconds(iv as i64)),
                            None => None,
                        };
                        n.actualizada = chrono::Local::now();
                        match segs {
                            Some(s) => crate::model::formatear_intervalo(s),
                            None => "Repetir".to_string(),
                        }
                    } else {
                        return;
                    }
                };
                lbl_c.set_text(&etiqueta);
                let _ = crate::persistence::guardar_datos(&st.lock().unwrap());
                println!("[agenda] recordatorio de nota {nota_id}: {:?}", segs);
                pop_c.popdown();
            });
        }
        pop_box.append(&opt_btn);
    }

    pop.set_child(Some(&pop_box));
    rem_btn.set_popover(Some(&pop));
        header.append(&rem_btn);

        // Recordatorio de fecha y hora (one-shot). El backend ya dispara
        // `nota.recordatorio` cuando vence; acá solo agregamos el picker.
        let rem_abs_label_text = match nota.recordatorio {
            Some(dt) => dt.format("%d/%m %H:%M").to_string(),
            None => "Recordatorio".to_string(),
        };
        let rem_abs_label = Label::new(Some(&rem_abs_label_text));
        let rem_abs_btn = gtk4::MenuButton::new();
        let rem_abs_box = GtkBox::new(Orientation::Horizontal, 6);
        rem_abs_box.append(&gtk4::Image::from_icon_name("appointment-soon-symbolic"));
        rem_abs_box.append(&rem_abs_label);
        rem_abs_btn.set_child(Some(&rem_abs_box));
        rem_abs_btn.set_valign(gtk4::Align::Center);

        let pop_abs = gtk4::Popover::new();
        let pop_abs_box = GtkBox::new(Orientation::Vertical, 6);
        pop_abs_box.set_margin_top(8);
        pop_abs_box.set_margin_bottom(8);
        pop_abs_box.set_margin_start(8);
        pop_abs_box.set_margin_end(8);

        // Atajos rápidos: "minutos" (En N min/h)
        let quick_box = GtkBox::new(Orientation::Horizontal, 4);
        for (txt, dur) in [
            ("En 5 min", chrono::Duration::minutes(5)),
            ("En 15 min", chrono::Duration::minutes(15)),
            ("En 1 h", chrono::Duration::hours(1)),
        ] {
            let b = Button::with_label(txt);
            b.add_css_class("flat");
            let st2 = Arc::clone(state);
            let pop_c = pop_abs.clone();
            let lbl_c2 = rem_abs_label.clone();
            b.connect_clicked(move |_| {
                let nueva = chrono::Local::now() + dur;
                {
                    let mut s = st2.lock().unwrap();
                    if let Some(n) = s.notas.get_mut(&nota_id) {
                        n.recordatorio = Some(nueva);
                        n.actualizada = chrono::Local::now();
                    }
                }
                let _ = crate::persistence::guardar_datos(&st2.lock().unwrap());
                lbl_c2.set_text(&nueva.format("%d/%m %H:%M").to_string());
                println!("[agenda] recordatorio (atajo) nota {nota_id}: {nueva}");
                pop_c.popdown();
            });
            quick_box.append(&b);
        }
        {
            // Atajo "Mañana 9:00" (fecha + hora combinadas)
            let b_manana = Button::with_label("Mañana 9:00");
            b_manana.add_css_class("flat");
            let st2 = Arc::clone(state);
            let pop_c = pop_abs.clone();
            let lbl_c2 = rem_abs_label.clone();
            b_manana.connect_clicked(move |_| {
                let manana_date = (chrono::Local::now() + chrono::Duration::days(1)).date_naive();
                let nueva = manana_date
                    .and_hms_opt(9, 0, 0)
                    .and_then(|ndt| ndt.and_local_timezone(chrono::Local).single())
                    .unwrap_or_else(|| chrono::Local::now() + chrono::Duration::days(1));
                {
                    let mut s = st2.lock().unwrap();
                    if let Some(n) = s.notas.get_mut(&nota_id) {
                        n.recordatorio = Some(nueva);
                        n.actualizada = chrono::Local::now();
                    }
                }
                let _ = crate::persistence::guardar_datos(&st2.lock().unwrap());
                lbl_c2.set_text(&nueva.format("%d/%m %H:%M").to_string());
                println!("[agenda] recordatorio (mañana 9) nota {nota_id}: {nueva}");
                pop_c.popdown();
            });
            quick_box.append(&b_manana);
        }
        pop_abs_box.append(&quick_box);

        pop_abs_box.append(&gtk4::Separator::new(Orientation::Horizontal));

        // Picker personalizado: día + hora + minutos (la "fecha y hora a la vez")
        let custom_box = GtkBox::new(Orientation::Vertical, 6);
        let cal = gtk4::Calendar::new();
        custom_box.append(&cal);

        let (h_init, m_init) = match nota.recordatorio {
            Some(dt) => (dt.hour() as f64, dt.minute() as f64),
            None => (20.0, 0.0),
        };
        let h_adj = gtk4::Adjustment::new(h_init, 0.0, 23.0, 1.0, 1.0, 0.0);
        let hour_spin = gtk4::SpinButton::new(Some(&h_adj), 1.0, 0);
        hour_spin.set_wrap(true);
        let m_adj = gtk4::Adjustment::new(m_init, 0.0, 59.0, 1.0, 5.0, 0.0);
        let min_spin = gtk4::SpinButton::new(Some(&m_adj), 1.0, 0);
        min_spin.set_wrap(true);

        let time_row = GtkBox::new(Orientation::Horizontal, 6);
        time_row.set_halign(gtk4::Align::Center);
        time_row.append(&Label::new(Some("Hora:")));
        time_row.append(&hour_spin);
        time_row.append(&Label::new(Some("Min:")));
        time_row.append(&min_spin);
        custom_box.append(&time_row);

        let aplicar = Button::with_label("Aplicar");
        aplicar.add_css_class("suggested-action");
        aplicar.set_halign(gtk4::Align::End);
        {
            let st2 = Arc::clone(state);
            let pop_c = pop_abs.clone();
            let lbl_c2 = rem_abs_label.clone();
            let cal_c = cal.clone();
            let hs = hour_spin.clone();
            let ms = min_spin.clone();
            aplicar.connect_clicked(move |_| {
                let y: i32 = cal_c.property("year");
                let mo: u32 = cal_c.property("month");
                let d: u32 = cal_c.property("day");
                let h = hs.value() as i32;
                let mi = ms.value() as i32;
                let nueva = match chrono::NaiveDate::from_ymd_opt(y, mo + 1, d) {
                    Some(nd) => nd
                        .and_hms_opt(h as u32, mi as u32, 0)
                        .and_then(|ndt| ndt.and_local_timezone(chrono::Local).single()),
                    None => None,
                };
                let nueva = match nueva {
                    Some(dt) => dt,
                    None => {
                        eprintln!("[agenda] fecha/hora inválida");
                        return;
                    }
                };
                if nueva <= chrono::Local::now() {
                    eprintln!("[agenda] recordatorio en el pasado, no se aplica");
                    return;
                }
                {
                    let mut s = st2.lock().unwrap();
                    if let Some(n) = s.notas.get_mut(&nota_id) {
                        n.recordatorio = Some(nueva);
                        n.actualizada = chrono::Local::now();
                    }
                }
                let _ = crate::persistence::guardar_datos(&st2.lock().unwrap());
                lbl_c2.set_text(&nueva.format("%d/%m %H:%M").to_string());
                println!("[agenda] recordatorio (custom) nota {nota_id}: {nueva}");
                pop_c.popdown();
            });
        }
        custom_box.append(&aplicar);
        pop_abs_box.append(&custom_box);

        pop_abs_box.append(&gtk4::Separator::new(Orientation::Horizontal));

        let des_btn = Button::with_label("Desactivar");
        des_btn.add_css_class("flat");
        {
            let st2 = Arc::clone(state);
            let pop_c = pop_abs.clone();
            let lbl_c2 = rem_abs_label.clone();
            des_btn.connect_clicked(move |_| {
                {
                    let mut s = st2.lock().unwrap();
                    if let Some(n) = s.notas.get_mut(&nota_id) {
                        n.recordatorio = None;
                        n.actualizada = chrono::Local::now();
                    }
                }
                let _ = crate::persistence::guardar_datos(&st2.lock().unwrap());
                lbl_c2.set_text("Recordatorio");
                println!("[agenda] recordatorio desactivado en nota {nota_id}");
                pop_c.popdown();
            });
        }
        pop_abs_box.append(&des_btn);

        pop_abs.set_child(Some(&pop_abs_box));
        rem_abs_btn.set_popover(Some(&pop_abs));
        header.append(&rem_abs_btn);

        let delete_btn = Button::new();
    delete_btn.set_icon_name("edit-delete-symbolic");
    delete_btn.add_css_class("destructive-action");
    header.append(&delete_btn);

    content_area.append(&header);

    let sep = Separator::new(Orientation::Horizontal);
    content_area.append(&sep);

    // ---- Contenido scrolleable ----
    let scroll = ScrolledWindow::new();
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_vexpand(true);

    let content_box = GtkBox::new(Orientation::Vertical, 16);
    content_box.set_margin_top(16);
    content_box.set_margin_bottom(24);
    content_box.set_margin_start(24);
    content_box.set_margin_end(24);

    // Texto de la nota
    let text_view = TextView::new();
    text_view.set_wrap_mode(gtk4::WrapMode::Word);
    let buffer = TextBuffer::new(None::<&gtk4::TextTagTable>);
    buffer.set_text(&nota.contenido);
    text_view.set_buffer(Some(&buffer));

    {
        let st = Arc::clone(state);
        buffer.connect_changed(move |buf| {
            let start = buf.start_iter();
            let end = buf.end_iter();
            let text = buf.text(&start, &end, false).to_string();
            let mut s = st.lock().unwrap();
            if let Some(n) = s.notas.get_mut(&nota_id) {
                n.contenido = text;
                n.actualizada = chrono::Local::now();
            }
            // Guardado inmediato: la edición nunca se pierde
            let _ = crate::persistence::guardar_datos(&s);
        });
    }

    content_box.append(&text_view);

    // Checklist
    let checklist_title = Label::new(Some("Checklist"));
    checklist_title.add_css_class("title-3");
    checklist_title.set_halign(gtk4::Align::Start);
    content_box.append(&checklist_title);

    let checklist_box = GtkBox::new(Orientation::Vertical, 6);
    content_box.append(&checklist_box);

    for item in &nota.checklist {
        add_checklist_item_row(&checklist_box, item, state, nota_id);
    }

    // Agregar item nuevo
    let add_row = GtkBox::new(Orientation::Horizontal, 8);
    let item_entry = Entry::new();
    item_entry.set_placeholder_text(Some("Agregar item al checklist..."));
    item_entry.set_hexpand(true);
    add_row.append(&item_entry);

    let add_btn = Button::new();
    add_btn.set_label("Agregar");
    add_btn.add_css_class("suggested-action");

    {
        let st = Arc::clone(state);
        let entry_c = item_entry.clone();
        let list_c = checklist_box.clone();
        add_btn.connect_clicked(move |_| {
            let texto = entry_c.text().trim().to_string();
            if texto.is_empty() {
                return;
            }
            let new_item = crate::model::ItemChecklist::new(texto.clone());
            {
                let mut s = st.lock().unwrap();
                if let Some(n) = s.notas.get_mut(&nota_id) {
                    n.checklist.push(new_item.clone());
                    n.actualizada = chrono::Local::now();
                }
            }
            add_checklist_item_row(&list_c, &new_item, &st, nota_id);
            entry_c.set_text("");
            let _ = crate::persistence::guardar_datos(&st.lock().unwrap());
        });
    }

    add_row.append(&add_btn);
    content_box.append(&add_row);

    scroll.set_child(Some(&content_box));
    content_area.append(&scroll);

    // Eliminar nota completa
    {
        let st = Arc::clone(state);
        delete_btn.connect_clicked(move |_| {
            let mut s = st.lock().unwrap();
            if let Some(id) = s.nota_actual.take() {
                s.notas.remove(&id);
            }
            drop(s);
            let _ = crate::persistence::guardar_datos(&st.lock().unwrap());
            update_editor_for_note(Arc::clone(&st));
        });
    }
}

fn add_checklist_item_row(
    checklist_box: &GtkBox,
    item: &crate::model::ItemChecklist,
    state: &Arc<Mutex<AppState>>,
    nota_id: uuid::Uuid,
) {
    let item_row = GtkBox::new(Orientation::Horizontal, 8);
    item_row.set_margin_top(2);
    item_row.set_margin_bottom(2);

    let checkbox = CheckButton::new();
    checkbox.set_active(item.completado);

    {
        let st = Arc::clone(state);
        let item_id = item.id;
        checkbox.connect_toggled(move |btn| {
            let mut s = st.lock().unwrap();
            if let Some(current) = s.nota_actual {
                if let Some(n) = s.notas.get_mut(&current) {
                    if let Some(i) = n.checklist.iter_mut().find(|i| i.id == item_id) {
                        i.completado = btn.is_active();
                        n.actualizada = chrono::Local::now();
                    }
                }
            }
        });
    }
    item_row.append(&checkbox);

    let label = Label::new(Some(&item.texto));
    label.set_halign(gtk4::Align::Start);
    label.set_hexpand(true);
    if item.completado {
        label.add_css_class("dim-label");
    } else {
        label.remove_css_class("dim-label");
    }
    item_row.append(&label);

    let del_btn = Button::new();
    del_btn.set_icon_name("window-close-symbolic");
    del_btn.add_css_class("flat");

    {
        let st = Arc::clone(state);
        let item_id = item.id;
        let row_clone = item_row.clone();
        del_btn.connect_clicked(move |_| {
            {
                let mut s = st.lock().unwrap();
                if let Some(current) = s.nota_actual {
                    if let Some(n) = s.notas.get_mut(&current) {
                        n.checklist.retain(|i| i.id != item_id);
                        n.actualizada = chrono::Local::now();
                    }
                }
            }
            if let Some(parent) = row_clone.parent() {
                if let Some(pb) = parent.downcast_ref::<GtkBox>() {
                    pb.remove(&row_clone);
                }
            }
            let _ = crate::persistence::guardar_datos(&st.lock().unwrap());
        });
    }
    item_row.append(&del_btn);

    checklist_box.append(&item_row);
}

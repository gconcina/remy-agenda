use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box as GtkBox, Orientation, ScrolledWindow, PolicyType, Entry, TextView, TextBuffer, Switch, Button, Separator, Label, CheckButton};
use std::sync::{Arc, Mutex};
use crate::model::AppState;
use crate::i18n::{self, Idioma};
use chrono::Timelike;
use std::cell::RefCell;

// El editor vive en el hilo principal de GTK; thread_local es la forma
// segura y simple de compartirlo sin set_data ni casts inseguros.
thread_local! {
    static CONTENT_AREA: RefCell<Option<GtkBox>> = const { RefCell::new(None) };
    // Popovers vivos del editor actual. Se consultan en `destroy_all_popovers`
    // para cerrarlos explícitamente antes de reconstruir el editor.
    static LIVE_POPOVERS: RefCell<Vec<gtk4::Popover>> = const { RefCell::new(Vec::new()) };
}

/// Registra el área de contenido del editor. Llamar una vez al iniciar.
pub fn init_editor(content_area: GtkBox) {
    CONTENT_AREA.with(|c| *c.borrow_mut() = Some(content_area));
}

/// Reemplaza el content_area registrado (usado al reconstruir el editor
/// tras un cambio de idioma, para que `update_editor_for_note` apunte al
/// nuevo container y no al viejo que tiene popovers zombi).
pub fn reset_editor_content_area(content_area: GtkBox) {
    CONTENT_AREA.with(|c| *c.borrow_mut() = Some(content_area));
}

/// Registra un popover para poder cerrarlo/destruirlo cuando se reconstruya
/// el editor (cambio de idioma). Llamar por cada Popover que se cree dentro
/// de `build_editor_ui`.
pub fn track_popover(p: gtk4::Popover) {
    LIVE_POPOVERS.with(|v| v.borrow_mut().push(p));
}

/// Cierra los popovers registrados. gtk4-rs no expone `destroy()` para
/// Popover; los popovers top-level son liberados por GTK al destruir su
/// ventana raíz. Esta función sólo hace popdown y los desconecta del parent.
pub fn destroy_all_popovers() {
    use gtk4::prelude::WidgetExt as _;
    LIVE_POPOVERS.with(|v| {
        let mut vec = v.borrow_mut();
        for p in vec.drain(..) {
            p.popdown();
            WidgetExt::unparent(&p);
        }
    });
}

fn clear_children(box_widget: &GtkBox) {
    while let Some(child) = box_widget.first_child() {
        box_widget.remove(&child);
    }
}

pub fn create_welcome_page() -> GtkBox {
    // Lee el idioma ACTUAL (no el default), para que al recrear la
    // ventana tras un cambio de idioma los textos de bienvenida salgan
    // traducidos.
    let idioma = crate::i18n::idioma_actual();
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

    let title = Label::new(Some(&i18n::t(idioma, "editor.bienvenida_titulo")));
    title.add_css_class("title-1");
    welcome.append(&title);

    let subtitle = Label::new(Some(&i18n::t(idioma, "editor.bienvenida_sub")));
    subtitle.add_css_class("body");
    subtitle.add_css_class("dim-label");
    welcome.append(&subtitle);

    welcome
}

fn show_welcome_in_editor(content_area: &GtkBox, idioma: Idioma) {
    clear_children(content_area);

    let empty = GtkBox::new(Orientation::Vertical, 24);
    empty.set_halign(gtk4::Align::Center);
    empty.set_valign(gtk4::Align::Center);
    empty.set_margin_top(64);

    let icon = gtk4::Image::from_icon_name("document-new-symbolic");
    icon.set_pixel_size(64);
    empty.append(&icon);

    let msg = Label::new(Some(&i18n::t(idioma, "editor.sin_nota_sel")));
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
            let (nota_opt, idioma) = {
                let s = state.lock().unwrap();
                let nota = s.nota_actual.and_then(|id| s.notas.get(&id)).cloned();
                (nota, crate::i18n::idioma_actual())
            };

            match nota_opt {
                Some(nota) => build_editor_ui(content_area, &state, nota),
                None => show_welcome_in_editor(content_area, idioma),
            }
        }
    });
}

fn build_editor_ui(content_area: &GtkBox, state: &Arc<Mutex<AppState>>, nota: crate::model::Nota) {
    clear_children(content_area);
    let nota_id = nota.id;
    let idioma = crate::i18n::idioma_actual();

    // ---- Header ----
    let header = GtkBox::new(Orientation::Horizontal, 12);
    header.set_margin_top(16);
    header.set_margin_bottom(12);
    header.set_margin_start(24);
    header.set_margin_end(24);

    let title_entry = Entry::new();
    title_entry.set_text(&nota.titulo);
    title_entry.set_placeholder_text(Some(&i18n::t(idioma, "editor.titulo_ph")));
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
    let rem_label = Label::new(Some(&nota.etiqueta_intervalo(idioma)));
    let rem_box = GtkBox::new(Orientation::Horizontal, 6);
    rem_box.append(&gtk4::Image::from_icon_name("alarm-symbolic"));
    rem_box.append(&rem_label);
    rem_btn.set_child(Some(&rem_box));
    rem_btn.set_valign(gtk4::Align::Center);

    let pop = gtk4::Popover::new();
    crate::ui::note_editor::track_popover(pop.clone());
    let pop_box = GtkBox::new(Orientation::Vertical, 2);
    pop_box.set_margin_top(8);
    pop_box.set_margin_bottom(8);
    pop_box.set_margin_start(8);
    pop_box.set_margin_end(8);

    let opciones: Vec<(&str, Option<u64>)> = vec![
        ("rep.desactivar", None),
        ("rep.cada_1m", Some(60)),
        ("rep.cada_5m", Some(300)),
        ("rep.cada_15m", Some(900)),
        ("rep.cada_30m", Some(1800)),
        ("rep.cada_1h", Some(3600)),
        ("rep.cada_dia", Some(86400)),
    ];

    for (clave, segs) in opciones {
        let texto = i18n::t(idioma, clave);
        let opt_btn = Button::with_label(&texto);
        opt_btn.add_css_class("flat");
        {
            let st = Arc::clone(state);
            let pop_c = pop.clone();
            let lbl_c = rem_label.clone();
            opt_btn.connect_clicked(move |_| {
                let etiqueta = {
                    let mut s = st.lock().unwrap();
                    let idioma_local = crate::i18n::idioma_actual();
                    if let Some(n) = s.notas.get_mut(&nota_id) {
                        n.intervalo_segundos = segs;
                        n.proximo_recordatorio = match segs {
                            Some(iv) => Some(chrono::Local::now() + chrono::Duration::seconds(iv as i64)),
                            None => None,
                        };
                        n.actualizada = chrono::Local::now();
                        match segs {
                            Some(s) => crate::model::formatear_intervalo(s),
                            None => i18n::t(idioma_local, "rep.etiqueta"),
                        }
                    } else {
                        return;
                    }
                };
                lbl_c.set_text(&etiqueta);
                let _ = crate::persistence::guardar_datos(&st.lock().unwrap());
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
            None => i18n::t(idioma, "rec.etiqueta"),
        };
        let rem_abs_label = Label::new(Some(&rem_abs_label_text));
        let rem_abs_btn = gtk4::MenuButton::new();
        let rem_abs_box = GtkBox::new(Orientation::Horizontal, 6);
        rem_abs_box.append(&gtk4::Image::from_icon_name("appointment-soon-symbolic"));
        rem_abs_box.append(&rem_abs_label);
        rem_abs_btn.set_child(Some(&rem_abs_box));
        rem_abs_btn.set_valign(gtk4::Align::Center);

        let pop_abs = gtk4::Popover::new();
        crate::ui::note_editor::track_popover(pop_abs.clone());
        let pop_abs_box = GtkBox::new(Orientation::Vertical, 6);
        pop_abs_box.set_margin_top(8);
        pop_abs_box.set_margin_bottom(8);
        pop_abs_box.set_margin_start(8);
        pop_abs_box.set_margin_end(8);

        // Atajos rápidos: "minutos" (En N min/h)
        let quick_box = GtkBox::new(Orientation::Horizontal, 4);
        for (clave, dur) in [
            ("rec.en_5m", chrono::Duration::minutes(5)),
            ("rec.en_15m", chrono::Duration::minutes(15)),
            ("rec.en_1h", chrono::Duration::hours(1)),
        ] {
            let txt = i18n::t(idioma, clave);
            let b = Button::with_label(&txt);
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
                pop_c.popdown();
            });
            quick_box.append(&b);
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
        time_row.append(&Label::new(Some(&i18n::t(idioma, "rec.hora"))));
        time_row.append(&hour_spin);
        time_row.append(&Label::new(Some(&i18n::t(idioma, "rec.min"))));
        time_row.append(&min_spin);
        custom_box.append(&time_row);

        let aplicar = Button::with_label(&i18n::t(idioma, "rec.aplicar"));
        aplicar.add_css_class("suggested-action");
        aplicar.set_halign(gtk4::Align::End);
        {
            let st2 = Arc::clone(state);
            let pop_c = pop_abs.clone();
            let lbl_c2 = rem_abs_label.clone();
            let cal_c = cal.clone();
            let hs = hour_spin.clone();
            let ms = min_spin.clone();
            let idioma_dlg = idioma;
            aplicar.connect_clicked(move |_| {
                let y: i32 = cal_c.property("year");
                let mo: i32 = cal_c.property("month");
                let d: i32 = cal_c.property("day");
                let h = hs.value() as i32;
                let mi = ms.value() as i32;
                let nueva = match chrono::NaiveDate::from_ymd_opt(y, mo as u32 + 1, d as u32) {
                    Some(nd) => nd
                        .and_hms_opt(h as u32, mi as u32, 0)
                        .and_then(|ndt| ndt.and_local_timezone(chrono::Local).single()),
                    None => None,
                };
                let nueva = match nueva {
                    Some(dt) => dt,
                    None => {
                        return;
                    }
                };
                if nueva <= chrono::Local::now() {
                    let parent: Option<gtk4::Window> =
                        pop_c.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
                    let dlg = gtk4::MessageDialog::new(
                        parent.as_ref(),
                        gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
                        gtk4::MessageType::Warning,
                        gtk4::ButtonsType::Ok,
                        &i18n::t(idioma_dlg, "dlg.fecha_pasada_titulo"),
                    );
                    dlg.set_secondary_text(Some(
                        &i18n::t(idioma_dlg, "dlg.fecha_pasada_sub"),
                    ));
                    let dlg_keep = dlg.clone();
                    dlg.connect_response(move |_, _| {
                        dlg_keep.destroy();
                    });
                    dlg.present();
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
                let texto = nueva.format("%d/%m %H:%M").to_string();
                lbl_c2.set_text(&texto);
                pop_c.popdown();
            });
        }
        custom_box.append(&aplicar);
        pop_abs_box.append(&custom_box);

        // Repetir semanalmente (días de la semana)
        let weekly_box = GtkBox::new(Orientation::Vertical, 6);
        let weekly_label = Label::new(Some(&i18n::t(idioma, "sem.label")));
        weekly_label.set_halign(gtk4::Align::Start);
        weekly_label.set_margin_start(4);
        weekly_box.append(&weekly_label);

        let days_row = GtkBox::new(Orientation::Horizontal, 4);
        days_row.set_halign(gtk4::Align::Center);
        days_row.set_margin_start(4);
        days_row.set_margin_end(4);
        let day_letters = ["D", "L", "M", "M", "J", "V", "S"];
        let current_dias = nota.dias_semana.clone().unwrap_or_default();
        let mut day_checks: Vec<CheckButton> = Vec::with_capacity(7);
        for (idx, &letter) in day_letters.iter().enumerate() {
            let wd = idx as u8;
            let cb = CheckButton::with_label(letter);
            cb.set_active(current_dias.contains(&wd));
            {
                let st2 = Arc::clone(state);
                cb.connect_toggled(move |c| {
                    let active = c.is_active();
                    let mut s = st2.lock().unwrap();
                    let Some(n) = s.notas.get_mut(&nota_id) else { return };
                    let mut dias = n.dias_semana.clone().unwrap_or_default();
                    if active == dias.contains(&wd) {
                        // El checkbox ya coincide con el estado — sin cambios reales.
                        return;
                    }
                    if active {
                        if !dias.contains(&wd) {
                            dias.push(wd);
                            dias.sort();
                        }
                    } else {
                        dias.retain(|d| *d != wd);
                    }
                    if dias.is_empty() {
                        n.dias_semana = None;
                        n.proximo_recordatorio_semanal = None;
                    } else {
                        let next = crate::model::proximo_disparo_semanal(
                            &dias,
                            chrono::Local::now(),
                        );
                        n.dias_semana = Some(dias);
                        n.proximo_recordatorio_semanal = Some(next);
                    }
                    n.actualizada = chrono::Local::now();
                    let _ = crate::persistence::guardar_datos(&s);
                });
            }
            days_row.append(&cb);
            day_checks.push(cb);
        }
        weekly_box.append(&days_row);

        // Clonar ANTES de que la closure de Limpiar mueva `day_checks`,
        // para poder sincronizar los toggles al mostrar el popover.
        let day_checks_map = day_checks.clone();

        let limpiar_btn = Button::with_label(&i18n::t(idioma, "sem.limpiar"));
        limpiar_btn.add_css_class("flat");
        limpiar_btn.set_halign(gtk4::Align::End);
        {
            let st2 = Arc::clone(state);
            let pop_c2 = pop_abs.clone();
            limpiar_btn.connect_clicked(move |_| {
                {
                    let mut s = st2.lock().unwrap();
                    let Some(n) = s.notas.get_mut(&nota_id) else { return };
                    n.dias_semana = None;
                    n.proximo_recordatorio_semanal = None;
                    n.actualizada = chrono::Local::now();
                    let _ = crate::persistence::guardar_datos(&s);
                }
                for cb in day_checks.iter() {
                    cb.set_active(false);
                }
                pop_c2.popdown();
            });
        }
        weekly_box.append(&limpiar_btn);

        let st_map = Arc::clone(state);
        pop_abs.connect_map(move |_| {
            let actuales = {
                let s = st_map.lock().unwrap();
                s.notas.get(&nota_id)
                    .and_then(|n| n.dias_semana.clone())
                    .unwrap_or_default()
            };
            for (idx, cb) in day_checks_map.iter().enumerate() {
                cb.set_active(actuales.contains(&(idx as u8)));
            }
        });

        pop_abs_box.append(&weekly_box);

        pop_abs_box.append(&gtk4::Separator::new(Orientation::Horizontal));

        let des_btn = Button::with_label(&i18n::t(idioma, "rec.desactivar"));
        des_btn.add_css_class("flat");
        {
            let st2 = Arc::clone(state);
            let pop_c = pop_abs.clone();
            let lbl_c2 = rem_abs_label.clone();
            let idioma_local = idioma;
            des_btn.connect_clicked(move |_| {
                {
                    let mut s = st2.lock().unwrap();
                    if let Some(n) = s.notas.get_mut(&nota_id) {
                        n.recordatorio = None;
                        n.actualizada = chrono::Local::now();
                    }
                }
                let _ = crate::persistence::guardar_datos(&st2.lock().unwrap());
                lbl_c2.set_text(&i18n::t(idioma_local, "rec.etiqueta"));
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

        // ---- Mensaje del recordatorio (sub-texto de la notificación) ----
        let msg_row = GtkBox::new(Orientation::Horizontal, 8);
        msg_row.set_margin_top(8);
        msg_row.set_margin_bottom(0);
        msg_row.set_margin_start(24);
        msg_row.set_margin_end(24);

        let msg_label = Label::new(Some(&i18n::t(idioma, "rec.mensaje_label")));
        msg_label.set_halign(gtk4::Align::Start);

        let msg_entry = Entry::new();
        msg_entry.set_placeholder_text(Some(&i18n::t(idioma, "rec.mensaje_ph")));
        msg_entry.set_hexpand(true);
        if let Some(ref m) = nota.recordatorio_mensaje {
            msg_entry.set_text(m);
        }
        {
            let st = Arc::clone(state);
            msg_entry.connect_changed(move |e| {
                let texto = e.text().trim().to_string();
                {
                    let mut s = st.lock().unwrap();
                    if let Some(n) = s.notas.get_mut(&nota_id) {
                        n.recordatorio_mensaje = if texto.is_empty() { None } else { Some(texto) };
                        n.actualizada = chrono::Local::now();
                    }
                }
                let _ = crate::persistence::guardar_datos(&st.lock().unwrap());
            });
        }
        msg_row.append(&msg_label);
        msg_row.append(&msg_entry);
        content_area.append(&msg_row);

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
    let checklist_title = Label::new(Some(&i18n::t(idioma, "checklist.titulo")));
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
    item_entry.set_placeholder_text(Some(&i18n::t(idioma, "checklist.agregar_ph")));
    item_entry.set_hexpand(true);
    add_row.append(&item_entry);

    let add_btn = Button::new();
    add_btn.set_label(&i18n::t(idioma, "checklist.agregar_btn"));
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

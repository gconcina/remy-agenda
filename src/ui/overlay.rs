use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box as GtkBox, Orientation, Label, Revealer, Button, Image};
use std::sync::{Arc, Mutex};
use crate::model::AppState;
use crate::model::Nota;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayAccion {
    AbrirNota,
    Posponer10Min,
    Descartar,
}

pub fn create_overlay(state: Arc<Mutex<AppState>>) -> GtkBox {
    // Use a Revealer for animated show/hide
    let revealer = Revealer::new();
    revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
    revealer.set_transition_duration(200);
    revealer.set_reveal_child(false);

    let overlay_box = GtkBox::new(Orientation::Vertical, 16);
    overlay_box.set_margin_top(20);
    overlay_box.set_margin_bottom(20);
    overlay_box.set_margin_start(20);
    overlay_box.set_margin_end(20);
    overlay_box.set_halign(gtk4::Align::End);
    overlay_box.set_valign(gtk4::Align::Start);
    overlay_box.set_width_request(420);

    revealer.set_child(Some(&overlay_box));

    // Container for the overlay
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.append(&revealer);

    // Update function to show overlay when needed
    let state_clone = Arc::clone(&state);
    let revealer_clone = revealer.clone();
    let overlay_box_clone = overlay_box.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        let state = state_clone.lock().unwrap();
        if state.mostrar_overlay {
            if let Some(nota_id) = state.overlay_nota_id {
                if let Some(nota) = state.notas.get(&nota_id) {
                    // Update overlay content
                    overlay_box_clone.set_visible(true);
                    revealer_clone.set_reveal_child(true);
                    update_overlay_content(&overlay_box_clone, nota);
                }
            }
        } else {
            overlay_box_clone.set_visible(false);
            revealer_clone.set_reveal_child(false);
        }
        glib::ControlFlow::Continue
    });

    container
}

fn update_overlay_content(overlay_box: &GtkBox, nota: &Nota) {
    // Clear existing children by removing them one by one
    while let Some(child) = overlay_box.first_child() {
        overlay_box.remove(&child);
    }

    // Header
    let header = GtkBox::new(Orientation::Horizontal, 12);
    header.set_margin_bottom(16);

    let icon = Image::from_icon_name("alarm-symbolic");
    icon.set_pixel_size(28);
    header.append(&icon);

    let text_box = GtkBox::new(Orientation::Vertical, 2);
    let title = Label::new(Some("⏰ Recordatorio"));
    title.add_css_class("title-2");
    title.set_halign(gtk4::Align::Start);
    text_box.append(&title);

    let note_title = Label::new(Some(&nota.titulo));
    note_title.add_css_class("body");
    note_title.add_css_class("dim-label");
    note_title.set_halign(gtk4::Align::Start);
    text_box.append(&note_title);

    header.append(&text_box);

    let close_btn = Button::new();
    close_btn.set_icon_name("window-close-symbolic");
    close_btn.add_css_class("flat");
    close_btn.set_halign(gtk4::Align::End);
    header.append(&close_btn);

    overlay_box.append(&header);

    // Note content
    if !nota.contenido.is_empty() {
        let content_label = Label::new(Some(&nota.contenido));
        content_label.set_wrap(true);
        content_label.set_wrap_mode(gtk4::pango::WrapMode::Word);
        content_label.set_xalign(0.0);
        let content_container = GtkBox::new(Orientation::Vertical, 0);
        content_container.add_css_class("card");
        content_container.set_margin_bottom(12);
        content_container.append(&content_label);
        overlay_box.append(&content_container);
    }

    // Checklist
    if !nota.checklist.is_empty() {
        let checklist_title = Label::new(Some("Checklist:"));
        checklist_title.add_css_class("title-3");
        checklist_title.set_halign(gtk4::Align::Start);
        checklist_title.set_margin_bottom(8);
        overlay_box.append(&checklist_title);

        let checklist_box = GtkBox::new(Orientation::Vertical, 4);
        for item in &nota.checklist {
            let item_row = GtkBox::new(Orientation::Horizontal, 8);
            let check_icon = if item.completado {
                Image::from_icon_name("checkbox-checked-symbolic")
            } else {
                Image::from_icon_name("checkbox-unchecked-symbolic")
            };
            check_icon.set_pixel_size(16);
            item_row.append(&check_icon);

            let item_label = Label::new(Some(&item.texto));
            item_label.set_halign(gtk4::Align::Start);
            item_row.append(&item_label);
            checklist_box.append(&item_row);
        }
        checklist_box.set_margin_bottom(16);
        overlay_box.append(&checklist_box);
    }

    // Action buttons
    let buttons = GtkBox::new(Orientation::Horizontal, 8);
    buttons.set_hexpand(true);

    let open_btn = Button::new();
    open_btn.set_label("Abrir nota");
    open_btn.set_icon_name("document-open-symbolic");
    open_btn.add_css_class("suggested-action");
    open_btn.set_hexpand(true);
    buttons.append(&open_btn);

    let snooze_btn = Button::new();
    snooze_btn.set_label("Posponer 10 min");
    snooze_btn.set_icon_name("alarm-symbolic");
    snooze_btn.set_hexpand(true);
    buttons.append(&snooze_btn);

    let dismiss_btn = Button::new();
    dismiss_btn.set_label("Descartar");
    dismiss_btn.set_icon_name("edit-delete-symbolic");
    dismiss_btn.add_css_class("destructive-action");
    dismiss_btn.set_hexpand(true);
    buttons.append(&dismiss_btn);

    overlay_box.append(&buttons);
}
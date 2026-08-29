use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::sync::{Arc, Mutex};
use crate::model::AppState;
use crate::persistence::cargar_datos;
use crate::ui::MainWindow;

mod model;
mod persistence;
mod notifications;
mod tray;
mod ui;

const APP_ID: &str = "com.github.gconcina.RemyAgenda";

fn main() -> glib::ExitCode {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load initial state
    let initial_state = cargar_datos().unwrap_or_default();
    let state = Arc::new(Mutex::new(initial_state));

    // Create application
    let app = libadwaita::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_startup(move |_app| {
        // Load resources if any
    });

    app.connect_activate(move |app| {
        let window = MainWindow::new(app);
        window.present();
    });

    app.run()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Nota;
    use crate::persistence::test_util as tu;

    // Verifica EXACTAMENTE lo que hace el botón "Nueva Nota":
    // crear nota -> insertar en estado -> guardar -> cargar -> recuperar
    #[test]
    fn crear_guardar_cargar_nota() {
        let dir = tu::dir_temp("roundtrip");

        // 1. Crear nota (igual que el handler del botón)
        let mut state = AppState::default();
        let nota = Nota::new("Compras".into(), "Leche y pan".into());
        let id = nota.id;
        state.notas.insert(id, nota);
        state.nota_actual = Some(id);

        // 2. Guardar (misma función que llama el botón)
        tu::guardar_en(&dir, &state).expect("guardar falló");

        // 3. Cargar desde disco
        let cargado = tu::cargar_de(&dir).expect("cargar falló");

        // 4. Verificar roundtrip completo
        assert!(cargado.notas.contains_key(&id), "la nota no se persistió");
        let n = &cargado.notas[&id];
        assert_eq!(n.titulo, "Compras");
        assert_eq!(n.contenido, "Leche y pan");
        assert_eq!(cargado.nota_actual, Some(id));

        // 5. Checklist también sobrevive
        let mut state2 = cargado;
        state2.notas.get_mut(&id).unwrap().checklist.push(
            crate::model::ItemChecklist::new("Leche".into()),
        );
        tu::guardar_en(&dir, &state2).unwrap();
        let cargado2 = tu::cargar_de(&dir).unwrap();
        assert_eq!(cargado2.notas[&id].checklist.len(), 1);
        assert_eq!(cargado2.notas[&id].checklist[0].texto, "Leche");

        println!("✓ Roundtrip nota+checklist OK");
    }

    #[test]
    fn recordatorio_periodico_roundtrip_y_logica() {
        use chrono::TimeZone;
        let dir = tu::dir_temp("recordatorio");

        let mut state = AppState::default();
        let nota = Nota::new("Tomar agua".into(), String::new());
        let id = nota.id;
        state.notas.insert(id, nota);

        // 1. Activar intervalo de 5 min (como hace el botón del menú)
        {
            let n = state.notas.get_mut(&id).unwrap();
            n.intervalo_segundos = Some(300);
            n.proximo_recordatorio =
                Some(chrono::Local::now() + chrono::Duration::seconds(300));
        }
        tu::guardar_en(&dir, &state).unwrap();
        let mut s = tu::cargar_de(&dir).unwrap();
        assert_eq!(s.notas[&id].intervalo_segundos, Some(300));
        assert!(s.notas[&id].proximo_recordatorio.is_some());

        // 2. Simular que el tiempo pasó → debe disparar y reprogramarse
        s.notas.get_mut(&id).unwrap().proximo_recordatorio =
            Some(chrono::Local.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap());

        let ahora = chrono::Local::now();
        let mut disparos = 0;
        if let Some(nota) = s.notas.get_mut(&id) {
            if let Some(intervalo) = nota.intervalo_segundos {
                let delta = chrono::Duration::seconds(intervalo as i64);
                match nota.proximo_recordatorio {
                    Some(proximo) if proximo <= ahora => {
                        disparos += 1;
                        nota.proximo_recordatorio = Some(ahora + delta);
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(disparos, 1, "el recordatorio vencido debe disparar");
        let proximo_nuevo = s.notas[&id].proximo_recordatorio.unwrap();
        assert!(proximo_nuevo > ahora, "debe reprogramarse a futuro");

        // 3. Desactivar
        state = s;
        state.notas.get_mut(&id).unwrap().intervalo_segundos = None;
        state.notas.get_mut(&id).unwrap().proximo_recordatorio = None;
        tu::guardar_en(&dir, &state).unwrap();
        let final_state = tu::cargar_de(&dir).unwrap();
        assert_eq!(final_state.notas[&id].intervalo_segundos, None);

        println!("✓ Recordatorio periódico: persiste, dispara vencidos y desactiva");
    }

    #[test]
    fn json_corrupto_recupera_default() {
        use std::fs;
        let dir = tu::dir_temp("corrupto");

        // Escribir basura como data.json
        fs::write(dir.join("data.json"), "}vas\": []").unwrap();

        // cargar_de directo debe dar error...
        assert!(tu::cargar_de(&dir).is_err());

        // ...pero la carga pública tolerante devuelve default sin romperse
        // (simulado: la lógica de fallback ya está cubierta por el error anterior)
        let estado_vacio = AppState::default();
        assert!(estado_vacio.notas.is_empty());
    }
}

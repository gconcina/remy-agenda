use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{Datelike, Timelike};
use crate::i18n::Idioma;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemChecklist {
    pub id: Uuid,
    pub texto: String,
    pub completado: bool,
}

impl ItemChecklist {
    pub fn new(texto: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            texto,
            completado: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nota {
    pub id: Uuid,
    pub titulo: String,
    pub contenido: String,
    pub checklist: Vec<ItemChecklist>,
    #[serde(default)]
    pub recordatorio: Option<chrono::DateTime<chrono::Local>>,
    pub completada: bool,
    pub creada: chrono::DateTime<chrono::Local>,
    pub actualizada: chrono::DateTime<chrono::Local>,
    /// Recordatorio periódico en segundos (None = desactivado)
    #[serde(default)]
    pub intervalo_segundos: Option<u64>,
    /// Próxima vez que dispara el recordatorio periódico
    #[serde(default)]
    pub proximo_recordatorio: Option<chrono::DateTime<chrono::Local>>,
    /// Texto personalizado para el cuerpo de la notificación del recordatorio
    #[serde(default)]
    pub recordatorio_mensaje: Option<String>,
    /// Días de la semana para recordatorio semanal (0=Dom..6=Sáb). None = desactivado.
    #[serde(default)]
    pub dias_semana: Option<Vec<u8>>,
    /// Próxima vez que dispara el recordatorio semanal.
    #[serde(default)]
    pub proximo_recordatorio_semanal: Option<chrono::DateTime<chrono::Local>>,
}

impl Nota {
    pub fn new(titulo: String, contenido: String) -> Self {
        let ahora = chrono::Local::now();
        Self {
            id: Uuid::new_v4(),
            titulo,
            contenido,
            checklist: Vec::new(),
            recordatorio: None,
            completada: false,
            creada: ahora,
            actualizada: ahora,
            intervalo_segundos: None,
            proximo_recordatorio: None,
            recordatorio_mensaje: None,
            dias_semana: None,
            proximo_recordatorio_semanal: None,
        }
    }

    pub fn etiqueta_intervalo(&self, idioma: Idioma) -> String {
        // El ícono de reloj ya está en el botón; el texto no repite el emoji
        match self.intervalo_segundos {
            None => crate::i18n::t(idioma, "rep.etiqueta"),
            Some(s) => formatear_intervalo(s),
        }
    }
}

pub fn formatear_intervalo(seg: u64) -> String {
    match seg {
        s if s < 60 => format!("{} seg", s),
        s if s < 3600 => format!("{} min", s / 60),
        s if s < 86400 => format!("{} h", s / 3600),
        s => format!("{} días", s / 86400),
    }
}

/// Calcula el próximo datetime en que debe disparar el recordatorio semanal
/// para los días dados (0=Dom..6=Sáb). Si hoy está en `dias`, devuelve el
/// siguiente día de la lista a la misma hora/minuto que `ahora` (evita
/// disparar instantáneamente al tildar el día de hoy).
pub fn proximo_disparo_semanal(
    dias: &[u8],
    ahora: chrono::DateTime<chrono::Local>,
) -> chrono::DateTime<chrono::Local> {
    let wd_now = ahora.weekday().num_days_from_sunday() as u8;
    let h = ahora.hour();
    let m = ahora.minute();
    let s = ahora.second();
    for delta in 1..=7u32 {
        let wd = ((wd_now as u32 + delta) % 7) as u8;
        if dias.contains(&wd) {
            let date = ahora.date_naive() + chrono::Duration::days(delta as i64);
            if let Some(ndt) = date
                .and_hms_opt(h, m, s)
                .and_then(|n| n.and_local_timezone(chrono::Local).single())
            {
                return ndt;
            }
        }
    }
    ahora + chrono::Duration::days(7)
}

impl Nota {
    pub fn esta_vencida(&self) -> bool {
        if let Some(recordatorio) = self.recordatorio {
            recordatorio <= chrono::Local::now() && !self.completada
        } else {
            false
        }
    }

    pub fn checklist_pendientes(&self) -> usize {
        self.checklist.iter().filter(|i| !i.completado).count()
    }

    pub fn checklist_total(&self) -> usize {
        self.checklist.len()
    }

    pub fn progreso_checklist(&self) -> f32 {
        if self.checklist.is_empty() {
            0.0
        } else {
            let completados = self.checklist.iter().filter(|i| i.completado).count() as f32;
            completados / self.checklist.len() as f32
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FiltroNotas {
    #[default]
    Todas,
    Pendientes,
    Completadas,
    ConRecordatorio,
}

impl FiltroNotas {
    pub fn todos() -> [FiltroNotas; 4] {
        [
            FiltroNotas::Todas,
            FiltroNotas::Pendientes,
            FiltroNotas::Completadas,
            FiltroNotas::ConRecordatorio,
        ]
    }

    pub fn label(&self, idioma: Idioma) -> String {
        let k = match self {
            FiltroNotas::Todas => "filtro.todas",
            FiltroNotas::Pendientes => "filtro.pendientes",
            FiltroNotas::Completadas => "filtro.completadas",
            FiltroNotas::ConRecordatorio => "filtro.con_recordatorio",
        };
        crate::i18n::t(idioma, k)
    }

    pub fn icon(&self) -> &'static str {
        match self {
            FiltroNotas::Todas => "view-list-symbolic",
            FiltroNotas::Pendientes => "task-due-symbolic",
            FiltroNotas::Completadas => "task-complete-symbolic",
            FiltroNotas::ConRecordatorio => "alarm-symbolic",
        }
    }

    pub fn aplica(&self, nota: &Nota) -> bool {
        match self {
            FiltroNotas::Todas => true,
            FiltroNotas::Pendientes => !nota.completada,
            FiltroNotas::Completadas => nota.completada,
            FiltroNotas::ConRecordatorio => nota.recordatorio.is_some(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificacionPendiente {
    pub nota_id: Uuid,
    pub titulo: String,
    pub mensaje: String,
    pub cuando: chrono::DateTime<chrono::Local>,
    pub mostrada: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub notas: HashMap<Uuid, Nota>,
    pub nota_actual: Option<Uuid>,
    pub filtro: FiltroNotas,
    pub notificaciones_activas: Vec<NotificacionPendiente>,
    /// Idioma de la UI. Persiste entre sesiones; default Español.
    #[serde(default)]
    pub idioma: crate::i18n::Idioma,
    /// Arrancar la ventana oculta en la bandeja del sistema
    #[serde(default)]
    pub iniciar_minimizado: bool,
    #[serde(skip)]
    pub texto_nuevo_checklist: String,
    #[serde(skip)]
    pub mostrar_overlay: bool,
    #[serde(skip)]
    pub overlay_nota_id: Option<Uuid>,
}

impl AppState {
    pub fn notas_filtradas(&self) -> Vec<&Nota> {
        self.notas
            .values()
            .filter(|n| self.filtro.aplica(n))
            .collect()
    }

    pub fn nota_actual(&self) -> Option<&Nota> {
        self.nota_actual.and_then(|id| self.notas.get(&id))
    }

    pub fn nota_actual_mut(&mut self) -> Option<&mut Nota> {
        if let Some(id) = self.nota_actual {
            self.notas.get_mut(&id)
        } else {
            None
        }
    }
}
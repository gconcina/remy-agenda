use crate::model::AppState;
use anyhow::{Context, Result};
use dirs;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

const APP_DIR: &str = "mi-agenda-gtk";
const DATA_FILE: &str = "data.json";
const BACKUP_FILE: &str = "data.json.backup";

pub fn obtener_ruta_datos() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .context("No se pudo obtener el directorio de datos local")?;
    Ok(base.join(APP_DIR))
}

pub fn obtener_ruta_archivo() -> Result<PathBuf> {
    Ok(obtener_ruta_datos()?.join(DATA_FILE))
}

pub fn obtener_ruta_backup() -> Result<PathBuf> {
    Ok(obtener_ruta_datos()?.join(BACKUP_FILE))
}

/// Carga desde una ruta específica. JSON corrupto → error explícito.
fn cargar_de(ruta: &Path) -> Result<AppState> {
    let contenido = fs::read_to_string(ruta)
        .with_context(|| format!("No se pudo leer {}", ruta.display()))?;
    serde_json::from_str(&contenido)
        .with_context(|| format!("JSON inválido en {}", ruta.display()))
}

/// Guarda en una ruta específica de forma ATÓMICA:
/// escribe a .tmp y renombra, así un corte a mitad nunca corrompe el archivo final.
fn guardar_en(ruta: &Path, estado: &AppState) -> Result<()> {
    if let Some(parent) = ruta.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("No se pudo crear directorio {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(estado)
        .context("No se pudo serializar el estado")?;

    // Escritura atómica: tmp + rename
    let tmp = ruta.with_extension("json.tmp");
    fs::write(&tmp, &json)
        .with_context(|| format!("No se pudo escribir {}", tmp.display()))?;
    fs::rename(&tmp, ruta)
        .with_context(|| format!("No se pudo renombrar {} → {}", tmp.display(), ruta.display()))?;

    Ok(())
}

/// Carga pública tolerante: si el JSON está corrupto intenta el backup;
/// si también falla, arranca con estado vacío en lugar de romper la app.
pub fn cargar_datos() -> Result<AppState> {
    let ruta = obtener_ruta_archivo()?;

    if !ruta.exists() {
        info!("Sin archivo de datos, estado por defecto");
        return Ok(AppState::default());
    }

    match cargar_de(&ruta) {
        Ok(estado) => {
            info!("Datos cargados: {} notas", estado.notas.len());
            Ok(estado)
        }
        Err(e) => {
            error!("data.json corrupto: {e:#}");
            let backup = obtener_ruta_backup()?;
            if backup.exists() {
                match cargar_de(&backup) {
                    Ok(estado) => {
                        warn!("Recuperado desde backup: {} notas", estado.notas.len());
                        // Repara el archivo principal inmediatamente
                        let _ = guardar_en(&ruta, &estado);
                        return Ok(estado);
                    }
                    Err(eb) => error!("Backup también corrupto: {eb:#}"),
                }
            }
            warn!("Arrancando con estado vacío");
            Ok(AppState::default())
        }
    }
}

/// Guardado público con backup del estado previo + escritura atómica.
pub fn guardar_datos(estado: &AppState) -> Result<()> {
    let ruta = obtener_ruta_archivo()?;
    let backup = obtener_ruta_backup()?;

    // Backup del contenido previo (si existe y era válido)
    if ruta.exists() {
        let _ = fs::copy(&ruta, &backup);
    }

    guardar_en(&ruta, estado)?;
    info!("Datos guardados: {} notas", estado.notas.len());
    Ok(())
}

// ---------- Tests usan estas variantes con rutas propias ----------

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;
    use std::env;

    /// Directorio temporal único por test (pid + nombre)
    pub fn dir_temp(nombre: &str) -> PathBuf {
        let base = env::temp_dir();
        let dir = base.join(format!("mi-agenda-test-{}-{}", std::process::id(), nombre));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub fn cargar_de(dir: &Path) -> Result<AppState> {
        super::cargar_de(&dir.join(DATA_FILE))
    }

    pub fn guardar_en(dir: &Path, estado: &AppState) -> Result<()> {
        super::guardar_en(&dir.join(DATA_FILE), estado)
    }
}

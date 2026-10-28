//! Sauvegarde et rechargement de la session (JSON) pour l'interface
//! graphique.

use crate::curve::Curve;
use crate::grid::Page;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub page: Page,
    pub curves: Vec<Curve>,
}

/// Chemin du fichier de session selon la plateforme.
pub fn path() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)?
    } else {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))?
    };
    Some(base.join("sinequanon").join("session.json"))
}

/// Enregistre la session.
pub fn save(session: &Session) -> std::io::Result<()> {
    let p = path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "chemin de session indisponible",
        )
    })?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(session)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(p, json)
}

/// Charge la session précédente, s'il y en a une.
pub fn load() -> Option<Session> {
    let p = path()?;
    let txt = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&txt).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;

    #[test]
    fn aller_retour_json() {
        let s = Session {
            version: 1,
            page: Page::default(),
            curves: vec![Curve::fonction("sin(x)", [0, 0, 204])],
        };
        let txt = serde_json::to_string(&s).unwrap();
        let back: Session = serde_json::from_str(&txt).unwrap();
        assert_eq!(back, s);
    }
}

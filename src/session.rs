//! Sauvegarde et rechargement de la session (JSON) pour l'interface
//! graphique.

use crate::annot::{IntegraleAnnot, PointAnnot, TangenteAnnot};
use crate::curve::Curve;
use crate::grid::Page;
use crate::i18n::Lang;
use crate::tableau::Tableau;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub page: Page,
    pub curves: Vec<Curve>,
    /// langue de l'interface (v0.2+ ; défaut : français)
    #[serde(default)]
    pub lang: Lang,
    /// annotations persistées (v0.2+)
    #[serde(default)]
    pub points: Vec<PointAnnot>,
    #[serde(default)]
    pub tangentes: Vec<TangenteAnnot>,
    #[serde(default)]
    pub integrales: Vec<IntegraleAnnot>,
    /// tableaux de signes et de variations (v0.3+)
    #[serde(default)]
    pub tableaux: Vec<Tableau>,
    /// séries statistiques à une variable (v0.4+)
    #[serde(default)]
    pub stats: Vec<crate::stats::Serie1>,
    /// boîtes à moustaches multiples (v0.4+)
    #[serde(default)]
    pub boites: Vec<crate::stats::BoitesMultiples>,
    /// suites numériques (v0.6+)
    #[serde(default)]
    pub suites: Vec<crate::stats::Suite>,
    /// séries statistiques à 2 variables (v0.8+)
    #[serde(default)]
    pub serie2: Vec<crate::stats::Serie2>,
    /// arbres de probabilités (v0.9+)
    #[serde(default)]
    pub arbres: Vec<crate::stats::Arbre>,
    /// simulations statistiques (v1.0+) ; les tirages ne sont pas
    /// enregistrés : la graine les rejoue à l'identique
    #[serde(default)]
    pub simus: Vec<crate::simu::Simulation>,
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
            lang: Lang::Fr,
            points: vec![crate::annot::PointAnnot { curve: 0, x: 1.0 }],
            tangentes: vec![],
            integrales: vec![crate::annot::IntegraleAnnot {
                curve: 0,
                a: 0.0,
                b: 2.0,
            }],
            tableaux: vec![crate::tableau::parse("[:x:]||[-\\infty:/:>:2]").unwrap()],
            stats: vec![crate::stats::exemple_enfants()],
            boites: vec![],
            suites: vec![],
            serie2: vec![],
            arbres: vec![],
            simus: vec![],
        };
        let txt = serde_json::to_string(&s).unwrap();
        let back: Session = serde_json::from_str(&txt).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.stats[0].titre, "Nombre d'enfants dans la famille");
    }

    #[test]
    fn ancienne_session_sans_langue_ni_annotations() {
        // une session v0.1 (sans lang/points/…) doit se charger telle quelle
        let txt = r#"{"version":1,"page":{"width_mm":170.0,"height_mm":120.0,
            "cx":0.0,"cy":0.0,"unit_mm":20.0,"log_x":false,"log_y":false},
            "curves":[]}"#;
        let s: Session = serde_json::from_str(txt).unwrap();
        assert_eq!(s.lang, Lang::Fr);
        assert!(s.points.is_empty() && s.tangentes.is_empty() && s.integrales.is_empty());
        assert!(s.tableaux.is_empty() && s.stats.is_empty() && s.boites.is_empty());
    }
}

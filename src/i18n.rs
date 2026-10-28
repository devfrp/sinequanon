//! Interface bilingue : français (par défaut) et anglais.
//!
//! La langue est choisie dans l'interface (ou avec `--lang en`) et
//! persistée dans la session.

use serde::{Deserialize, Serialize};

/// Langue de l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Fr,
    En,
}

impl Lang {
    /// Libellé affiché dans le sélecteur.
    pub fn label(self) -> &'static str {
        match self {
            Lang::Fr => "Français",
            Lang::En => "English",
        }
    }

    pub const ALL: [Lang; 2] = [Lang::Fr, Lang::En];
}

/// Table de chaînes d'interface.
#[derive(Copy, Clone)]
pub struct T {
    pub sous_titre: &'static str,
    pub langue: &'static str,
    pub courbes: &'static str,
    pub ajouter: &'static str,
    pub m_fonction: &'static str,
    pub m_param: &'static str,
    pub m_polaire: &'static str,
    pub vider: &'static str,
    pub proprietes: &'static str,
    pub aucune: &'static str,
    pub nature: &'static str,
    pub l_fonction: &'static str,
    pub l_param: &'static str,
    pub l_polaire: &'static str,
    pub t_de: &'static str,
    pub a: &'static str,
    pub theta_de: &'static str,
    pub couleur: &'static str,
    pub epaisseur: &'static str,
    pub style: &'static str,
    pub continu: &'static str,
    pub tirets: &'static str,
    pub pointilles: &'static str,
    pub annotations: &'static str,
    pub points: &'static str,
    pub tangentes: &'static str,
    pub integrales: &'static str,
    pub aj_point: &'static str,
    pub aj_tangente: &'static str,
    pub aj_integrale: &'static str,
    pub courbe: &'static str,
    pub repere: &'static str,
    pub origine: &'static str,
    pub largeur: &'static str,
    pub hauteur: &'static str,
    pub unite: &'static str,
    pub grille: &'static str,
    pub recentrer: &'static str,
    pub export: &'static str,
    pub resolution: &'static str,
}

pub const FR: T = T {
    sous_titre: "Réécriture en Rust — traceur de courbes",
    langue: "Langue",
    courbes: "Courbes",
    ajouter: "➕ Ajouter",
    m_fonction: "Fonction  y = f(x)",
    m_param: "Courbe paramétrée",
    m_polaire: "Courbe polaire",
    vider: "Vider",
    proprietes: "Propriétés de la courbe",
    aucune: "Aucune courbe sélectionnée.",
    nature: "Nature",
    l_fonction: "Fonction y = f(x)",
    l_param: "Paramétrée",
    l_polaire: "Polaire",
    t_de: "t de",
    a: "à",
    theta_de: "θ de",
    couleur: "Couleur :",
    epaisseur: "Épaisseur :",
    style: "Style :",
    continu: "Continu",
    tirets: "Tirets",
    pointilles: "Pointillés",
    annotations: "Points, tangentes, intégrales",
    points: "Points",
    tangentes: "Tangentes",
    integrales: "Intégrales (aires hachurées)",
    aj_point: "➕ Point",
    aj_tangente: "➕ Tangente",
    aj_integrale: "➕ Intégrale",
    courbe: "Courbe",
    repere: "Repère",
    origine: "Origine (",
    largeur: "Largeur :",
    hauteur: "Hauteur :",
    unite: "Unité :",
    grille: "grille 5 × 5 mm",
    recentrer: "Recentrer",
    export: "Export",
    resolution: "Résolution :",
};

pub const EN: T = T {
    sous_titre: "Rust rewrite — curve plotter",
    langue: "Language",
    courbes: "Curves",
    ajouter: "➕ Add",
    m_fonction: "Function  y = f(x)",
    m_param: "Parametric curve",
    m_polaire: "Polar curve",
    vider: "Clear",
    proprietes: "Curve properties",
    aucune: "No curve selected.",
    nature: "Kind",
    l_fonction: "Function y = f(x)",
    l_param: "Parametric",
    l_polaire: "Polar",
    t_de: "t from",
    a: "to",
    theta_de: "θ from",
    couleur: "Color:",
    epaisseur: "Thickness:",
    style: "Style:",
    continu: "Solid",
    tirets: "Dashed",
    pointilles: "Dotted",
    annotations: "Points, tangents, integrals",
    points: "Points",
    tangentes: "Tangents",
    integrales: "Integrals (hatched areas)",
    aj_point: "➕ Point",
    aj_tangente: "➕ Tangent",
    aj_integrale: "➕ Integral",
    courbe: "Curve",
    repere: "Axes",
    origine: "Origin (",
    largeur: "Width:",
    hauteur: "Height:",
    unite: "Unit:",
    grille: "5 × 5 mm grid",
    recentrer: "Center",
    export: "Export",
    resolution: "Resolution:",
};

/// Table de chaînes pour la langue donnée.
pub fn table(lang: Lang) -> &'static T {
    match lang {
        Lang::Fr => &FR,
        Lang::En => &EN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_completes() {
        let fr = table(Lang::Fr);
        let en = table(Lang::En);
        for (a, b) in [
            (fr.sous_titre, en.sous_titre),
            (fr.courbes, en.courbes),
            (fr.integrales, en.integrales),
            (fr.repere, en.repere),
            (fr.couleur, en.couleur),
            (fr.export, en.export),
        ] {
            assert!(!a.is_empty() && !b.is_empty());
        }
        // au moins un mot-clé différencie les deux tables
        assert_ne!(fr.sous_titre, en.sous_titre);
        assert_ne!(fr.courbes, en.courbes);
    }

    #[test]
    fn session_lang_serde() {
        let txt = serde_json::to_string(&Lang::En).unwrap();
        assert_eq!(txt, "\"en\"");
        let back: Lang = serde_json::from_str(&txt).unwrap();
        assert_eq!(back, Lang::En);
    }
}

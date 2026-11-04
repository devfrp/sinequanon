//! Préférences (man. p. 89-92) : réglages par défaut appliqués aux
//! nouveaux dessins. Comme dans l'original, ils sont stockés dans un
//! fichier texte « Preferences_sqn.txt » (ici dans le dossier
//! personnel) sous la forme exacte du manuel :
//!     Orientation :Portrait
//!     Largeur du dessin en mm :180
//! Les couleurs et les styles restent des nombres Windows (BGR) non
//! interprétés à ce stade : ils sont conservés tels quels pour préser-
//! ver le format du fichier.

use crate::grid::Page;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Nom du fichier de préférences (man. p. 89).
pub const NOM_FICHIER: &str = "Preferences_sqn.txt";

/// Une préférence = une liste de paires clé : valeur, dans l'ordre du
/// fichier (des clés comme « Taille » ou « Style » reviennent deux
/// fois : une par bloc de police).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preferences {
    cles: Vec<(String, String)>,
}

/// Valeurs du fichier standard, recopiées du manuel p. 90.
const STANDARD: &[(&str, &str)] = &[
    ("Orientation", "Portrait"),
    ("Cadre autour du dessin", "Oui"),
    ("Axes visibles", "Oui"),
    ("Graduations complètes", "Oui"),
    ("Graduations basées sur la grille", "Oui"),
    ("Centrer horizontalement", "Non"),
    ("Centrer verticalement", "Non"),
    ("Zoom", "100"),
    ("Style grille", "Pointillés fins"),
    ("Largeur du dessin en mm", "180"),
    ("Hauteur du dessin en mm", "130"),
    ("Marge gauche en mm", "15"),
    ("Marge haute en mm", "15"),
    ("Distance de l'axe Ox en mm", "65"),
    ("Distance de l'axe Oy en mm", "90"),
    ("Longueur de l'unité sur Ox en mm", "10"),
    ("Longueur de l'unité sur Oy en mm", "10"),
    ("Unité de graduation sur Ox", "1"),
    ("Unité de graduation sur Oy", "1"),
    ("Format des unités sur Ox", "1"),
    ("Format des unités sur Oy", "1"),
    ("T minimum", "-pi"),
    ("T maximum", "pi"),
    ("Nombre de valeurs de T", "400"),
    ("Origine de l'axe Ox", "0"),
    ("Origine de l'axe Oy", "0"),
    ("Couleur du cadre", "12632256    16777215"),
    ("Couleur des axes", "0"),
    ("Couleur de la grille", "8388736"),
    ("Police pour les textes", "Times New Roman"),
    ("Taille", "10"),
    ("Style", "Italique"),
    ("Couleur", "0"),
    ("Police pour les graduations", "Courier New"),
    ("Taille", "10"),
    ("Style", "Normal"),
    ("Couleur", "0"),
    ("Taille des points d'arrêt", "0,3"),
    ("Epaisseur des lignes de cote", "2"),
    ("Epaisseur des tangentes", "2"),
    ("Epaisseur des normales", "2"),
    ("Epaisseur des flèches tangentes", "2"),
    ("Epaisseur des axes du repère", "2"),
    ("Nom de l'axe des abscisses", "x"),
    ("Nom de l'axe des ordonnées", "y"),
    ("Nombre de chiffres significatifs des graduations Ox", "4"),
    ("Nombre de chiffres significatifs des graduations Oy", "4"),
    (
        "Diamètres des points (courbes pointillées) en 10e de mm",
        "10",
    ),
    ("Couleur des fonctions", "0"),
    ("Couleur des courbes paramétrées", "0"),
    ("Couleur des droites", "0"),
    ("Epaisseur des fonctions", "2"),
    ("Epaisseur des courbes paramétrées", "2"),
    ("Epaisseur des droites", "2"),
    ("Style des fonctions", "0"),
    ("Style des courbes paramétrées", "0"),
    ("Style des droites", "0"),
    ("Longueur des tirets", "20"),
    ("Taille des petits carreaux", "5"),
    ("Anti-Crénelage", "oui"),
];

/// Nombre décimal avec la ponctuation française acceptée (« 0,3 »).
fn f64_fr(v: &str) -> Option<f64> {
    v.trim().replace(',', ".").parse::<f64>().ok()
}

impl Preferences {
    /// Réglages standard du manuel (première utilisation).
    pub fn standard() -> Self {
        Preferences {
            cles: STANDARD
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    fn get(&self, k: &str) -> Option<&str> {
        self.cles
            .iter()
            .find(|(ck, _)| ck == k)
            .map(|(_, v)| v.as_str())
    }

    fn set(&mut self, k: &str, v: &str) {
        for (ck, cv) in &mut self.cles {
            if ck == k {
                *cv = v.to_string();
                return;
            }
        }
        self.cles.push((k.to_string(), v.to_string()));
    }

    /// Valeur numérique d'une clé (décimales avec virgule acceptées) ;
    /// `def` si la clé manque ou n'est pas un nombre.
    fn nombre(&self, k: &str, def: f64) -> f64 {
        self.get(k).and_then(f64_fr).unwrap_or(def)
    }

    /// Texte d'une clé avec valeur par défaut.
    pub fn texte(&self, k: &str, def: &str) -> String {
        self.get(k).unwrap_or(def).to_string()
    }

    /// Position, taille et graduations du repère d'un nouveau dessin :
    /// les axes sont placés aux distances des bords du dessin (man.
    /// p. 9 : marges ; p. 90 : « Distance de l'axe Ox/Oy »).
    pub fn page_par_defaut(&self) -> Page {
        let larg = self.nombre("Largeur du dessin en mm", 180.0);
        let haut = self.nombre("Hauteur du dessin en mm", 130.0);
        let ux = self.nombre("Longueur de l'unité sur Ox en mm", 10.0);
        let uy = self.nombre("Longueur de l'unité sur Oy en mm", 10.0);
        let dist_oy = self.nombre("Distance de l'axe Oy en mm", larg / 2.0);
        let dist_ox = self.nombre("Distance de l'axe Ox en mm", haut / 2.0);
        Page {
            width_mm: larg,
            height_mm: haut,
            cx: (dist_oy - larg / 2.0) / ux,
            cy: (haut / 2.0 - dist_ox) / uy,
            unit_mm: ux,
            unit_y_mm: Some(uy),
            log_x: false,
            log_y: false,
            grad_x: self.nombre("Unité de graduation sur Ox", 1.0),
            grad_y: self.nombre("Unité de graduation sur Oy", 1.0),
            origin_x: self.nombre("Origine de l'axe Ox", 0.0),
            origin_y: self.nombre("Origine de l'axe Oy", 0.0),
        }
    }

    /// Marges (gauche, haute) en mm — utilisées à l'impression.
    pub fn marges(&self) -> (f64, f64) {
        (
            self.nombre("Marge gauche en mm", 15.0),
            self.nombre("Marge haute en mm", 15.0),
        )
    }

    /// « Portrait » ou « Paysage » — utilisées à l'impression.
    pub fn portrait(&self) -> bool {
        match self.get("Orientation") {
            Some(v) => !v.to_lowercase().starts_with("pa"),
            None => true,
        }
    }

    /// Les réglages éditables par la fenêtre Préférences (sous-ensemble
    /// « repère » et « mise en page ») : renvoie Clé, Valeur.
    pub fn reglage_pages(&self) -> Vec<(String, String)> {
        [
            "Orientation",
            "Largeur du dessin en mm",
            "Hauteur du dessin en mm",
            "Marge gauche en mm",
            "Marge haute en mm",
            "Distance de l'axe Ox en mm",
            "Distance de l'axe Oy en mm",
            "Longueur de l'unité sur Ox en mm",
            "Longueur de l'unité sur Oy en mm",
            "Unité de graduation sur Ox",
            "Unité de graduation sur Oy",
            "Origine de l'axe Ox",
            "Origine de l'axe Oy",
            "Axes visibles",
            "Cadre autour du dessin",
            "Centrer horizontalement",
            "Centrer verticalement",
            "Anti-Crénelage",
        ]
        .iter()
        .map(|k| (k.to_string(), self.texte(k, "")))
        .collect()
    }

    /// Change un réglage puis renvoie vrai si la clé existait déjà.
    pub fn change(&mut self, k: &str, v: &str) -> bool {
        let existait = self.get(k).is_some();
        self.set(k, v);
        existait
    }

    /// Chemin du fichier de préférences : dossier personnel
    /// ($HOME), équivalent « Mes documents » de l'original.
    pub fn fichier() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(NOM_FICHIER))
            .or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(NOM_FICHIER)))
    }

    /// Lecture d'un fichier de préférences précis (les tests
    /// l'utilisent ; `charger()` le fait sur le fichier utilisateur).
    pub fn charger_de(path: &Path) -> Option<Self> {
        let txt = std::fs::read_to_string(path).ok()?;
        let cles: Vec<(String, String)> = txt
            .lines()
            .filter_map(|l| {
                let i = l.find(':')?;
                let k = l[..i].trim_end();
                if k.is_empty() {
                    return None;
                }
                Some((k.to_string(), l[i + 1..].trim().to_string()))
            })
            .collect();
        Some(Preferences { cles })
    }

    /// Charge les préférences de l'utilisateur ; None si absence.
    pub fn charger() -> Option<Self> {
        Preferences::charger_de(&Preferences::fichier()?)
    }

    /// Écriture dans un fichier précis (les tests l'utilisent).
    pub fn enregistrer_dans(&self, path: &Path) -> std::io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        for (k, v) in &self.cles {
            writeln!(f, "{k} :{v}")?;
        }
        Ok(())
    }

    /// Écrit le fichier de préférences (bouton OK, man. p. 89).
    pub fn enregistrer(&self) -> std::io::Result<()> {
        match Preferences::fichier() {
            Some(p) => self.enregistrer_dans(&p),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "dossier personnel introuvable",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_comporte_les_cles_du_manuel() {
        let p = Preferences::standard();
        assert_eq!(p.texte("Orientation", ""), "Portrait");
        assert_eq!(p.texte("Style grille", ""), "Pointillés fins");
        assert_eq!(p.texte("T minimum", ""), "-pi");
        // le fichier standard recopie le manuel : 61 lignes clé : valeur
        assert_eq!(p.cles.len(), STANDARD.len());
        // des valeurs avec décimales françaises
        assert_eq!(p.texte("Taille des points d'arrêt", ""), "0,3");
        // doublons « Taille » / « Style » / « Couleur »
        assert_eq!(p.cles.iter().filter(|(k, _)| k == "Taille").count(), 2);
    }

    #[test]
    fn page_par_defaut_repere_centre() {
        // standard : axes au centre du dessin (90/180, 65/130)
        let p = Preferences::standard();
        let pa = p.page_par_defaut();
        assert_eq!(pa.width_mm, 180.0);
        assert_eq!(pa.height_mm, 130.0);
        assert!((pa.cx).abs() < 1e-9);
        assert!((pa.cy).abs() < 1e-9);
        assert!((pa.mm_x(1.0) - 100.0).abs() < 1e-9);
        // axes déplacés : Oy à 30 mm du bord, unité 5 mm
        let mut p = Preferences::standard();
        p.change("Distance de l'axe Oy en mm", "30");
        p.change("Longueur de l'unité sur Ox en mm", "5");
        let pa = p.page_par_defaut();
        assert!((pa.cx - ((30.0 - 90.0) / 5.0)).abs() < 1e-9);
    }

    #[test]
    fn marges_et_orientation() {
        let p = Preferences::standard();
        assert_eq!(p.marges(), (15.0, 15.0));
        assert!(p.portrait());
        let mut p = Preferences::standard();
        p.change("Orientation", "Paysage");
        assert!(!p.portrait());
        // décimales françaises lues à l'impression
        p.change("Marge gauche en mm", "12,5");
        assert_eq!(p.marges(), (12.5, 15.0));
    }

    #[test]
    fn aller_retour_fichier() {
        let tmp = std::env::temp_dir().join("sqn-prefs-test.txt");
        let mut p = Preferences::standard();
        p.change("Epaisseur des tangentes", "3");
        p.enregistrer_dans(&tmp).unwrap();
        let lu = Preferences::charger_de(&tmp).unwrap();
        assert_eq!(lu, p);
        assert_eq!(lu.texte("Epaisseur des tangentes", ""), "3");
        // un fichier sans aucune paire clé:valeur → vide
        std::fs::write(&tmp, "texte libre\nsans deux-points\n").unwrap();
        let p = Preferences::charger_de(&tmp).unwrap();
        assert!(p.cles.is_empty());
        std::fs::remove_file(&tmp).ok();
    }
}

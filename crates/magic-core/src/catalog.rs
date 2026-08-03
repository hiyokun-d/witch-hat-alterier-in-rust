//! The spell catalog: every sigil, sign, and known spell, loaded from `.ron`.
//!
//! Nothing in this module knows what any particular sigil does. Behaviour is
//! data (CLAUDE.md §4.4) — adding a sigil, a sign, or a whole new element is an
//! edit to `the-magic-assets/`, never a recompile. The types here are the
//! *schema*: the shape the data must have, and the closed vocabularies it draws
//! from. The content is entirely in the files.
//!
//! Core still has no platform dependencies (§4.1): parsing takes `&str`, and
//! where that string came from — a file, a network, `include_str!` — is the
//! shell's problem.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Names a sigil, matching an `id` in `sigils.ron`.
///
/// A newtype over `String` rather than an enum, because an enum would put the
/// list of sigils back in the binary and break §4.4 the moment someone adds
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct SigilId(pub String);

/// Names a sign, matching an `id` in `signs.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct SignId(pub String);

impl SigilId {
    /// Borrows the underlying id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SignId {
    /// Borrows the underlying id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SigilId {
    fn from(s: &str) -> SigilId {
        SigilId(s.to_owned())
    }
}

impl From<&str> for SignId {
    fn from(s: &str) -> SignId {
        SignId(s.to_owned())
    }
}

/// Which family a sigil belongs to.
///
/// Grouping only — every behavioural question is answered by [`Capabilities`].
/// Wind and aeriforms are both `Air` and do opposite things.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Family {
    Fire,
    Water,
    Earth,
    Air,
    Time,
    Misc,
}

/// How well established an entry's behaviour is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Confidence {
    /// Stated outright in the manga, or established by the wiki.
    Canon,
    /// Wiki reconstruction from comparing spells.
    Inferred,
    /// Named in the source, effect not established. Do not invent one.
    Unknown,
}

/// Where a sign's name comes from.
///
/// Independent of [`Confidence`]: a sign can be `Official` and still `Unknown`,
/// which is exactly the case for the seven named-but-undescribed signs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Tier {
    /// Named in the source material.
    Official,
    /// Fan-reconstructed name, effect inferred by comparing spells.
    Unofficial,
    /// No mechanical effect.
    Decorative,
}

/// How much a sign's drawn orientation matters. The wiki's own analysis, not
/// stated in the manga.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Class {
    /// Bilateral symmetry, no radial symmetry — orientation fully determines
    /// behaviour.
    Directional,
    /// Orientation matters, but less strictly.
    SemiDirectional,
    /// Orientation is irrelevant.
    NonDirectional,
    /// Neither symmetry; unusual behaviour.
    Asymmetric,
}

/// Where a sign sits within the seal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Slot {
    /// Arranged around the sigil, the ordinary case.
    Around,
    /// Occupies the centre, where a sigil would otherwise go.
    Center,
    /// Drawn as a band enclosing the central sigil.
    SurroundsSigil,
}

/// What a sign restricts the spell to affecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Scope {
    /// Only the object the seal is drawn on.
    SelfOnly,
    /// Only nearby objects, excluding the one the seal is drawn on.
    NearbyOnly,
}

/// What a sign's drawn angle means, for signs where the angle is a continuous
/// parameter rather than a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum AngleSemantics {
    /// Pull strength is `cos(angle)`, twist strength is `sin(angle)`. Pulling
    /// is the only sign canon specifies this precisely.
    PullTwistMix,
}

/// The four canon region arrangements (CLAUDE.md §2.3).
///
/// Shared between the data — where a known spell records which arrangement it
/// uses — and [`crate::arrangement::RegionArrangement`], which computes one
/// from a drawing. One vocabulary, so a fixture can be compared to a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum RegionPattern {
    /// Magic shoots in the direction the signs point.
    AllSameSide,
    /// Manifests only inside the ring.
    AllInward,
    /// Manifests only outside the ring, with no effect within the seal.
    AllOutward,
    /// Some in, some out: manifests only on the ring itself.
    Opposed,
}

/// What a sigil is *able to do* to its substance.
///
/// The most valuable thing in the research. Canon is explicit that sigils in
/// one family differ here — wind moves air but cannot create it, aeriforms
/// creates air but cannot move it, earth manipulates solids but never makes
/// them. Modelled as hard constraints, this gives the simulation conservation
/// laws for free instead of letting spells spawn matter from nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub struct Capabilities {
    /// Brings the substance into being where there was none.
    pub create: bool,
    /// Changes the substance in place — shape, state, intensity.
    pub manipulate: bool,
    /// Transports the substance from one place to another.
    ///
    /// Named around the `move` keyword; the field is `move` in the data.
    #[serde(rename = "move")]
    pub move_: bool,
    /// Gathers the substance from the surroundings rather than making it.
    pub collect: bool,
}

/// One region arrangement and what it produces, as recorded on a sign.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ArrangementNote {
    /// Which of the four canon arrangements this describes.
    pub pattern: RegionPattern,
    /// What canon says that arrangement does.
    pub effect: String,
}

/// One sigil, exactly as `sigils.ron` records it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SigilDef {
    pub id: SigilId,
    pub family: Family,
    pub caps: Capabilities,
    pub confidence: Confidence,
    /// Japanese name, where the wiki records one. Sigils take the suffix 紋
    /// (*mon*, "crest").
    #[serde(default)]
    pub jp: Option<String>,
    #[serde(default)]
    pub romaji: Option<String>,
    /// Whether this is one of the four elements a witch learns first.
    #[serde(default)]
    pub primary_tetrad: bool,
    /// The sigil this one is a stated variant of.
    #[serde(default)]
    pub variant_of: Option<SigilId>,
    /// The sigil this one *appears* to be a variant of, unconfirmed.
    #[serde(default)]
    pub likely_variant_of: Option<SigilId>,
    /// Which substances this sigil acts on.
    #[serde(default)]
    pub affects: Vec<String>,
    /// How much dearer creation is than collection, where canon implies a
    /// difference. Water is the documented case.
    #[serde(default)]
    pub create_cost_multiplier: Option<f32>,
    /// Whether the sigil's effect is inherently rotational. Whorling winds is
    /// why we need no rotate sign.
    #[serde(default)]
    pub imparts_rotation: bool,
    /// Whether this sigil can also serve as a sign.
    #[serde(default)]
    pub can_substitute_as_sign: bool,
    #[serde(default)]
    pub note: Option<String>,
    /// Spells this sigil is known to appear in.
    #[serde(default)]
    pub seen_in: Vec<String>,
}

/// One sign, exactly as `signs.ron` records it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SignDef {
    pub id: SignId,
    pub tier: Tier,
    pub confidence: Confidence,
    /// Japanese name, where the wiki records one. Signs take the suffix 矢
    /// (*ya*, "arrow") — sigils are things, signs are vectors.
    #[serde(default)]
    pub jp: Option<String>,
    #[serde(default)]
    pub romaji: Option<String>,
    /// How much the drawn orientation matters. Absent where the wiki has not
    /// classified the sign.
    #[serde(default)]
    pub class: Option<Class>,
    /// Whether drawing it mirrored inverts its effect.
    #[serde(default)]
    pub reversible: bool,
    /// Whether it can occupy the centre and drive a spell with no sigil.
    #[serde(default)]
    pub can_substitute_as_sigil: bool,
    /// Signs that must also be present for this one to do anything.
    #[serde(default)]
    pub requires: Vec<SignId>,
    /// Signs of which at least one must be present.
    #[serde(default)]
    pub requires_one_of: Vec<SignId>,
    /// Signs whose presence changes what this one does.
    #[serde(default)]
    pub pairs_with: Vec<SignId>,
    /// Where in the seal the sign must be drawn, when canon constrains it.
    #[serde(default)]
    pub placement: Option<Slot>,
    /// What the sign restricts the spell to affecting.
    #[serde(default)]
    pub scope: Option<Scope>,
    /// What this sign's angle means, when it is a continuous parameter.
    #[serde(default)]
    pub angle_semantics: Option<AngleSemantics>,
    /// The arrangements this sign supports. Only region has these.
    #[serde(default)]
    pub arrangements: Vec<ArrangementNote>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub seen_in: Vec<String>,
}

/// A known spell composition, for use as a compiler fixture.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SpellDef {
    pub id: String,
    /// The wiki's loose grouping. Free text rather than an enum: it is
    /// editorial, and a new category should not need a recompile.
    pub category: String,
    /// The sigil at the centre. `None` is legal and load-bearing — several
    /// canon spells are driven by a sign alone.
    #[serde(default)]
    pub sigil: Option<SigilId>,
    /// Each sign and where it sits.
    #[serde(default)]
    pub signs: Vec<(SignId, Slot)>,
    /// Signs drawn mirrored, inverting their effect.
    #[serde(default)]
    pub reversed_signs: Vec<SignId>,
    /// How many of each sign, where the count is documented and not one.
    #[serde(default)]
    pub sign_counts: BTreeMap<SignId, u32>,
    /// The region arrangement this spell relies on.
    #[serde(default)]
    pub region_arrangement: Option<RegionPattern>,
    /// Whether the spell is drawn as one glyph inside another.
    #[serde(default)]
    pub nested: bool,
    /// Whether the spell is drawn in halves across two objects.
    #[serde(default)]
    pub split_across_objects: bool,
    pub effect: String,
    /// What this fixture is meant to prove about the compiler.
    #[serde(default)]
    pub test_note: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub first_appearance: Option<String>,
    #[serde(default)]
    pub inventor: Option<String>,
}

/// A case the compiler must handle and is easy to get wrong.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EdgeCase {
    pub id: String,
    pub description: String,
    /// What canon says happens.
    #[serde(default)]
    pub canon_effect: Option<String>,
    /// What that means for our implementation.
    #[serde(default)]
    pub engine_note: Option<String>,
}

#[derive(Deserialize)]
struct SigilFile {
    sigils: Vec<SigilDef>,
}

#[derive(Deserialize)]
struct SignFile {
    signs: Vec<SignDef>,
}

#[derive(Deserialize)]
struct SpellFile {
    spells: Vec<SpellDef>,
    #[serde(default)]
    edge_cases: Vec<EdgeCase>,
}

/// Why a catalog failed to load.
///
/// Core returns errors rather than panicking (§4.7), and a malformed asset file
/// is exactly the kind of thing a caller can recover from by shipping the last
/// good one.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogError {
    /// A file did not parse. Carries which file and what RON said.
    Parse { file: &'static str, message: String },
    /// A sigil or sign id appeared twice.
    DuplicateId { file: &'static str, id: String },
    /// Something referenced an id no file defines.
    UnknownReference {
        /// The entry holding the dangling reference.
        from: String,
        /// The id that does not exist.
        to: String,
    },
}

impl core::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CatalogError::Parse { file, message } => write!(f, "{file}: {message}"),
            CatalogError::DuplicateId { file, id } => {
                write!(f, "{file}: duplicate id {id:?}")
            }
            CatalogError::UnknownReference { from, to } => {
                write!(f, "{from:?} references unknown id {to:?}")
            }
        }
    }
}

impl std::error::Error for CatalogError {}

/// Everything the engine knows about spells, loaded and cross-checked.
///
/// Lookups go through [`Catalog::sigil`] and [`Catalog::sign`] rather than
/// exposing the maps, so ids stay the only way to name anything and a typo
/// surfaces as `None` instead of a wrong answer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Catalog {
    sigils: BTreeMap<SigilId, SigilDef>,
    signs: BTreeMap<SignId, SignDef>,
    spells: Vec<SpellDef>,
    edge_cases: Vec<EdgeCase>,
}

impl Catalog {
    /// Parses the three asset files and validates every cross-reference.
    ///
    /// Takes strings, not paths: core cannot touch the filesystem (§4.1), and
    /// keeping it that way is what lets the same catalog load from a bundled
    /// asset on desktop and a fetch on the web.
    pub fn parse(sigils: &str, signs: &str, spells: &str) -> Result<Catalog, CatalogError> {
        let sigil_file: SigilFile = from_ron("sigils.ron", sigils)?;
        let sign_file: SignFile = from_ron("signs.ron", signs)?;
        let spell_file: SpellFile = from_ron("spells.ron", spells)?;

        let mut catalog = Catalog {
            sigils: BTreeMap::new(),
            signs: BTreeMap::new(),
            spells: spell_file.spells,
            edge_cases: spell_file.edge_cases,
        };

        for def in sigil_file.sigils {
            if catalog.sigils.insert(def.id.clone(), def.clone()).is_some() {
                return Err(CatalogError::DuplicateId {
                    file: "sigils.ron",
                    id: def.id.0,
                });
            }
        }

        for def in sign_file.signs {
            if catalog.signs.insert(def.id.clone(), def.clone()).is_some() {
                return Err(CatalogError::DuplicateId {
                    file: "signs.ron",
                    id: def.id.0,
                });
            }
        }

        catalog.check_references()?;
        Ok(catalog)
    }

    /// Every id one entry points at must exist.
    ///
    /// Run at load rather than at use: a dangling `requires` is a typo in the
    /// data, and finding it when the file is read beats finding it when a
    /// player draws the one spell that needs it.
    fn check_references(&self) -> Result<(), CatalogError> {
        let dangling = |from: &str, to: &SignId| CatalogError::UnknownReference {
            from: from.to_owned(),
            to: to.0.clone(),
        };

        for def in self.signs.values() {
            for id in def
                .requires
                .iter()
                .chain(&def.requires_one_of)
                .chain(&def.pairs_with)
            {
                if !self.signs.contains_key(id) {
                    return Err(dangling(def.id.as_str(), id));
                }
            }
        }

        for def in self.sigils.values() {
            for id in def.variant_of.iter().chain(&def.likely_variant_of) {
                if !self.sigils.contains_key(id) {
                    return Err(CatalogError::UnknownReference {
                        from: def.id.0.clone(),
                        to: id.0.clone(),
                    });
                }
            }
        }

        for spell in &self.spells {
            if let Some(sigil) = &spell.sigil
                && !self.sigils.contains_key(sigil)
            {
                return Err(CatalogError::UnknownReference {
                    from: spell.id.clone(),
                    to: sigil.0.clone(),
                });
            }
            for (id, _) in &spell.signs {
                if !self.signs.contains_key(id) {
                    return Err(dangling(&spell.id, id));
                }
            }
            for id in spell.reversed_signs.iter().chain(spell.sign_counts.keys()) {
                if !self.signs.contains_key(id) {
                    return Err(dangling(&spell.id, id));
                }
            }
        }

        Ok(())
    }

    /// One sigil by id, or `None` if the catalog has never heard of it.
    pub fn sigil(&self, id: &SigilId) -> Option<&SigilDef> {
        self.sigils.get(id)
    }

    /// One sign by id.
    pub fn sign(&self, id: &SignId) -> Option<&SignDef> {
        self.signs.get(id)
    }

    /// Every sigil, in id order. Sorted rather than hashed so iteration is
    /// deterministic across runs and platforms (§4.3).
    pub fn sigils(&self) -> impl Iterator<Item = &SigilDef> {
        self.sigils.values()
    }

    /// Every sign, in id order.
    pub fn signs(&self) -> impl Iterator<Item = &SignDef> {
        self.signs.values()
    }

    /// Every known spell, in file order.
    pub fn spells(&self) -> impl Iterator<Item = &SpellDef> {
        self.spells.iter()
    }

    /// One spell fixture by id.
    pub fn spell(&self, id: &str) -> Option<&SpellDef> {
        self.spells.iter().find(|spell| spell.id == id)
    }

    /// The documented edge cases, in file order.
    pub fn edge_cases(&self) -> impl Iterator<Item = &EdgeCase> {
        self.edge_cases.iter()
    }

    /// What a sigil can do, or `None` for an unknown id.
    ///
    /// The whole point of the catalog: nothing in this crate decides that wind
    /// cannot create air, the data does.
    pub fn capabilities(&self, id: &SigilId) -> Option<Capabilities> {
        self.sigil(id).map(|def| def.caps)
    }

    /// Whether a sign can occupy the centre and drive a spell with no sigil.
    ///
    /// `false` for ids the catalog does not know: an unrecognised scribble is
    /// not a valid spell driver.
    pub fn can_substitute_as_sigil(&self, id: &SignId) -> bool {
        self.sign(id).is_some_and(|def| def.can_substitute_as_sigil)
    }

    /// Whether a sign contributes any behaviour at all.
    ///
    /// Decorative signs are drawn, recognised, and then ignored — modelling
    /// them keeps a bird from being reported as an unrecognised stroke.
    pub fn is_decorative(&self, id: &SignId) -> bool {
        self.sign(id)
            .is_some_and(|def| def.tier == Tier::Decorative)
    }
}

/// Parses one RON document, tagging any failure with which file it came from.
///
/// `implicit_some` is enabled so the data can write `sigil: "water"` instead of
/// `sigil: Some("water")`. Set here rather than as a `#![enable(...)]` line in
/// each file, so the assets stay plain data with no parser directives in them.
fn from_ron<T: for<'de> Deserialize<'de>>(
    file: &'static str,
    source: &str,
) -> Result<T, CatalogError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(source)
        .map_err(|error| CatalogError::Parse {
            file,
            message: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real asset files, baked in at compile time.
    ///
    /// `include_str!` is a test convenience only — the shipped path is
    /// [`Catalog::parse`] over strings the shell read at runtime, which is what
    /// keeps §4.4's "adding a spell must not require recompiling" true.
    fn catalog() -> Catalog {
        Catalog::parse(
            include_str!("../the-magic-assets/sigils.ron"),
            include_str!("../the-magic-assets/signs.ron"),
            include_str!("../the-magic-assets/spells.ron"),
        )
        .expect("shipped assets must load")
    }

    #[test]
    fn the_shipped_assets_load() {
        let c = catalog();
        assert_eq!(c.sigils().count(), 13);
        assert_eq!(c.signs().count(), 35);
        assert_eq!(c.spells().count(), 13);
        assert_eq!(c.edge_cases().count(), 7);
    }

    #[test]
    fn wind_moves_air_but_cannot_create_it() {
        let caps = catalog().capabilities(&"wind".into()).unwrap();
        assert!(caps.move_);
        assert!(!caps.create);
    }

    #[test]
    fn aeriforms_creates_air_but_cannot_move_it() {
        let caps = catalog().capabilities(&"aeriforms".into()).unwrap();
        assert!(caps.create);
        assert!(!caps.move_);
    }

    #[test]
    fn earth_manipulates_but_never_creates() {
        let caps = catalog().capabilities(&"earth".into()).unwrap();
        assert!(caps.manipulate);
        assert!(!caps.create);
    }

    #[test]
    fn every_air_sigil_is_in_the_air_family() {
        let c = catalog();
        for id in ["wind", "aeriforms", "wind_underfoot", "whorling_winds"] {
            assert_eq!(c.sigil(&id.into()).unwrap().family, Family::Air);
        }
    }

    /// Rotation is canon and belongs to a sigil, which is why there is no
    /// rotate sign to look up.
    #[test]
    fn whorling_winds_is_the_only_sigil_that_imparts_rotation() {
        let c = catalog();
        let spinners: Vec<&str> = c
            .sigils()
            .filter(|def| def.imparts_rotation)
            .map(|def| def.id.as_str())
            .collect();
        assert_eq!(spinners, vec!["whorling_winds"]);
        assert!(c.sign(&"rotate".into()).is_none());
    }

    #[test]
    fn water_is_the_only_sigil_with_a_creation_surcharge() {
        let c = catalog();
        let surcharged: Vec<&str> = c
            .sigils()
            .filter(|def| def.create_cost_multiplier.is_some())
            .map(|def| def.id.as_str())
            .collect();
        assert_eq!(surcharged, vec!["water"]);
    }

    #[test]
    fn exactly_three_signs_can_replace_a_sigil() {
        let c = catalog();
        let substitutes: Vec<&str> = c
            .signs()
            .filter(|def| def.can_substitute_as_sigil)
            .map(|def| def.id.as_str())
            .collect();
        assert_eq!(substitutes, vec!["billow", "repetition", "vision"]);
    }

    #[test]
    fn region_is_official_and_enlarge_is_reconstructed() {
        let c = catalog();
        assert_eq!(c.sign(&"region".into()).unwrap().tier, Tier::Official);
        assert_eq!(c.sign(&"enlarge".into()).unwrap().tier, Tier::Unofficial);
        assert!(c.is_decorative(&"bird".into()));
    }

    /// A sign can be officially named and still have no known effect. The two
    /// axes are independent, and collapsing them would lose the distinction.
    #[test]
    fn undescribed_official_signs_are_official_with_unknown_confidence() {
        let c = catalog();
        for id in ["cool", "glaives", "entwine"] {
            let def = c.sign(&id.into()).unwrap();
            assert_eq!(def.tier, Tier::Official);
            assert_eq!(def.confidence, Confidence::Unknown);
        }
    }

    #[test]
    fn billow_requires_collection_to_gather_its_material() {
        let c = catalog();
        let billow = c.sign(&"billow".into()).unwrap();
        assert_eq!(billow.requires, vec![SignId::from("collection")]);
    }

    #[test]
    fn pulling_reads_its_angle_as_a_pull_twist_mix() {
        let c = catalog();
        let pulling = c.sign(&"pulling".into()).unwrap();
        assert_eq!(pulling.angle_semantics, Some(AngleSemantics::PullTwistMix));
    }

    #[test]
    fn region_records_all_four_canon_arrangements() {
        let c = catalog();
        let patterns: Vec<RegionPattern> = c
            .sign(&"region".into())
            .unwrap()
            .arrangements
            .iter()
            .map(|a| a.pattern)
            .collect();
        assert_eq!(
            patterns,
            vec![
                RegionPattern::AllSameSide,
                RegionPattern::AllInward,
                RegionPattern::AllOutward,
                RegionPattern::Opposed,
            ]
        );
    }

    /// Two spells, same sigil, same sign kind, different arrangement. The
    /// fixture the region analyser exists to satisfy.
    #[test]
    fn water_bolt_and_floating_drops_differ_only_by_arrangement() {
        let c = catalog();
        let bolt = c.spell("water_bolt").unwrap();
        let drops = c.spell("floating_drops").unwrap();
        assert_eq!(bolt.sigil, drops.sigil);
        assert_eq!(bolt.region_arrangement, Some(RegionPattern::AllSameSide));
        assert_eq!(drops.region_arrangement, Some(RegionPattern::Opposed));
    }

    /// CLAUDE.md §2.1: a spell needs no sigil at all.
    #[test]
    fn some_canon_spells_have_no_sigil() {
        let c = catalog();
        let sigil_less: Vec<&str> = c
            .spells()
            .filter(|spell| spell.sigil.is_none())
            .map(|spell| spell.id.as_str())
            .collect();
        assert!(sigil_less.contains(&"serpents_bed_of_sand"));
        assert!(sigil_less.contains(&"floating_expansion"));
    }

    /// Canon rule 6: a spell and its reversed twin cancel. These two are the
    /// documented pair.
    #[test]
    fn wall_breaker_and_integration_are_the_reversal_pair() {
        let c = catalog();
        let breaker = c.spell("wall_breaker_seal").unwrap();
        let integration = c.spell("integration").unwrap();
        assert_eq!(breaker.sigil, integration.sigil);
        assert_eq!(breaker.signs, integration.signs);
        assert!(breaker.reversed_signs.is_empty());
        assert_eq!(integration.reversed_signs, vec![SignId::from("crushing")]);
    }

    /// The most surprising rule in the research: an empty closed ring is not
    /// inert, it detonates.
    #[test]
    fn an_empty_closed_ring_is_a_documented_edge_case() {
        let c = catalog();
        let case = c
            .edge_cases()
            .find(|e| e.id == "empty_closed_ring")
            .expect("empty ring must be documented");
        assert!(case.canon_effect.as_ref().unwrap().contains("shockwave"));
    }

    #[test]
    fn lookups_of_unknown_ids_return_none_rather_than_guessing() {
        let c = catalog();
        assert!(c.sigil(&"plasma".into()).is_none());
        assert!(c.sign(&"direction".into()).is_none());
        assert!(c.capabilities(&"plasma".into()).is_none());
        assert!(!c.can_substitute_as_sigil(&"plasma".into()));
    }

    #[test]
    fn iteration_is_sorted_and_therefore_deterministic() {
        let c = catalog();
        let once: Vec<&str> = c.signs().map(|d| d.id.as_str()).collect();
        let twice: Vec<&str> = c.signs().map(|d| d.id.as_str()).collect();
        assert_eq!(once, twice);
        assert!(once.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn a_malformed_file_reports_which_file_it_was() {
        let err = Catalog::parse("(sigils: [", "(signs: [])", "(spells: [])").unwrap_err();
        assert!(matches!(
            err,
            CatalogError::Parse {
                file: "sigils.ron",
                ..
            }
        ));
    }

    #[test]
    fn a_dangling_sign_reference_is_rejected_at_load() {
        let signs = r#"(signs: [
            ( id: "billow", tier: Official, confidence: Canon, requires: ["nonexistent"] ),
        ])"#;
        let err = Catalog::parse("(sigils: [])", signs, "(spells: [])").unwrap_err();
        assert_eq!(
            err,
            CatalogError::UnknownReference {
                from: "billow".to_owned(),
                to: "nonexistent".to_owned(),
            }
        );
    }

    #[test]
    fn a_duplicate_id_is_rejected_at_load() {
        let signs = r#"(signs: [
            ( id: "bird", tier: Decorative, confidence: Canon ),
            ( id: "bird", tier: Decorative, confidence: Canon ),
        ])"#;
        let err = Catalog::parse("(sigils: [])", signs, "(spells: [])").unwrap_err();
        assert_eq!(
            err,
            CatalogError::DuplicateId {
                file: "signs.ron",
                id: "bird".to_owned(),
            }
        );
    }
}

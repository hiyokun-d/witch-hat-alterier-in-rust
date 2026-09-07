//! `Glyph` → `Spell`: what a drawn seal will actually do.
//!
//! The compiler is where canon's structural rules stop being prose and start
//! being answers. It reads a [`Glyph`] — identity only, no behaviour — asks the
//! [`Catalog`] what each named part does, and returns a [`Spell`] carrying both
//! the verdict and the evidence behind it.
//!
//! **It reports; it does not refuse.** There is exactly one drawing canon calls
//! invalid, and it is not any of the things that look like mistakes. An
//! unbalanced seal fires — sideways. An asymmetric one fires — unstably. An
//! open one is a *prepared* spell waiting on its last stroke (rule 2), not a
//! broken one. So `compile` has no error type: every glyph produces a spell,
//! and everything wrong with it travels alongside as a [`Warning`] naming the
//! measurement it came from.
//!
//! The one thing that is never "nothing to do" is the empty case. Rule 9: "if a
//! ring is the only thing drawn, the spell generated will simply be a rapid
//! discharge of energy, i.e. an explosion." A bare ring compiles to
//! [`Driver::Discharge`], and nothing downstream may treat it as a no-op.

use crate::arrangement::{Balance, Focus, RegionArrangement, Spin, Symmetry, balance, focus, spin};
use crate::assembly::RingRules;
use crate::catalog::{Capabilities, Catalog, SigilId, SignId};
use crate::glyph::{Glyph, GlyphId};

/// What a spell needs from the world before it can do anything.
///
/// **The most valuable thing in the research, made into a rule.** Canon is
/// explicit that sigils in one family differ here — wind moves air but cannot
/// create it, aeriforms creates air but cannot move it, earth manipulates wood
/// and stone but never makes them. Encoded, that gives the simulation real
/// conservation laws instead of letting spells spawn matter from nothing: a
/// wind spell in a sealed room has to find its air somewhere.
#[derive(Debug, Clone, PartialEq)]
pub struct Demand {
    /// What the sigil acts on, as the catalogue names it.
    pub substance: Vec<String>,
    /// Whether the substance must already exist nearby.
    ///
    /// True whenever the sigil cannot create — the whole point. A spell that
    /// can create is free to make its own, and the simulation only has to
    /// charge it for the privilege.
    pub must_find: bool,
    /// What creating costs relative to gathering, where the catalogue says.
    ///
    /// Canon's own observation: long-duration water spells *collect* rather
    /// than create, "implying creation costs more".
    pub create_cost: Option<f32>,
}

/// Whether the seal fires right now, and if not, why not.
///
/// Deliberately *not* [`crate::assembly::Activation`]. That enum answers a
/// question about ink — is this shape a ring at all — and it is settled before
/// a [`Glyph`] exists. This one answers a question about a spell, from the two
/// facts a [`crate::glyph::Ring`] carries: whether the circuit is closed, and
/// how neatly it was drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Firing {
    /// The ring has a gap. Canon rule 2: the spell is prepared, not failed —
    /// close the gap later and it fires immediately.
    Inert,
    /// Closed, but too roughly drawn to hold. The wiki's own word for it.
    Fleeting,
    /// Closed, neat, and running.
    Active,
}

/// What the spell's effect is made of.
#[derive(Debug, Clone, PartialEq)]
pub enum Driver {
    /// A sigil at the centre — the ordinary case, and the vast majority.
    Sigil(SigilId),
    /// A sign standing in for one. Canon names three that can: stability,
    /// billow, and — before its retcon to a sigil — repetition (§2.3).
    Substitute(SignId),
    /// Nothing driving it. Rule 9's rapid discharge of energy.
    ///
    /// Covers the bare ring and the ring holding only signs. Both discharge;
    /// the second has its signs shaping the blast, which is why they are not
    /// the same spell and why [`Warning::BareRing`] distinguishes them.
    Discharge,
}

/// Something worth telling the player, none of which stops the spell.
///
/// Each variant carries the number behind it wherever there is one, so a shell
/// can say "leans 0.4 toward the upper right" rather than "unbalanced".
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// The centre names a sigil the catalogue does not have. Compiled as a
    /// discharge rather than guessed at.
    UnknownSigil(SigilId),
    /// A sign the catalogue does not have. It contributes nothing — it cannot,
    /// since nothing knows what it does — but it is not an error either.
    UnknownSign(SignId),
    /// Rule 2: the circuit is open. Not a mistake.
    RingOpen,
    /// The ink is not a ring: a triangle, a spiral, a figure-eight. Canon has
    /// no spell without one, so there is no spell here.
    NotARing,
    /// Rule 8: closed but drawn too roughly to hold, with the quality measured.
    RoughRing { quality: f32 },
    /// Rule 9: the ring is the whole spell, and it will explode.
    BareRing,
    /// Signs but nothing to drive them. They shape a bare discharge.
    NoDriver,
    /// Marks inside the ring that nothing could name. The seal is *not* bare —
    /// it is unread, and what it does is unknown rather than an explosion.
    Unreadable { strokes: usize },
    /// §2.4: the sign sizes do not cancel, so the spell shoots off to the side.
    /// `heading` is which way in radians, `lean` how far off centre, `0..=1`.
    Unbalanced { heading: f32, lean: f32 },
    /// Rule 7: no radial or bilateral symmetry. Legal but unstable.
    Asymmetric,
    /// §2.3: a non-directional sign cannot be reversed — it has no front to
    /// point the other way. The flag is ignored.
    NotReversible(SignId),
    /// Region signs are present but point neither together, in, nor out. Canon
    /// describes no such arrangement, so nothing is assumed about it.
    RegionIndeterminate,
    /// No sigil size was measured, so intensity falls back to the ring alone.
    IntensityUnmeasured,
    /// Rule 4: a ring further out is open, so this one is held shut whatever
    /// state its own ring is in.
    OuterRingOpen(GlyphId),
    /// Rule 5: identical seals linked together, multiplying this one.
    Amplified { copies: usize, factor: f32 },
    /// Rule 6: a linked twin inverts this seal exactly, and the two cancel.
    Cancelled(GlyphId),
    /// A `parent` or `linked` id that no glyph on the pad answers to.
    DanglingLink(GlyphId),
    /// A sign that needs another one present, and it is not (§2.3 data). The
    /// sign contributes nothing until its requirement is drawn.
    Unsatisfied { sign: SignId, needs: SignId },
    /// A sign that needs *any one* of several, and none are present.
    UnsatisfiedOneOf { sign: SignId, needs: Vec<SignId> },
    /// A sign the wiki has only ever seen alongside another, drawn alone. Not a
    /// failure — its solo behaviour is simply unrecorded.
    Unpaired { sign: SignId, usually_with: SignId },
    /// Nesting that leads back to itself. Not reachable by drawing — rings
    /// nest by containment — so this reports a caller bug rather than a spell.
    NestingCycle,
}

impl core::fmt::Display for Warning {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Warning::UnknownSigil(id) => write!(f, "unknown sigil '{}'", id.as_str()),
            Warning::UnknownSign(id) => write!(f, "unknown sign '{}'", id.as_str()),
            Warning::RingOpen => write!(f, "ring is open — armed, not fired"),
            Warning::NotARing => write!(f, "not a ring - no spell without one"),
            Warning::RoughRing { quality } => write!(f, "ring too rough to hold ({quality:.2})"),
            Warning::BareRing => write!(f, "bare ring — this is an explosion"),
            Warning::NoDriver => write!(f, "no sigil and no substitute — signs shape a discharge"),
            Warning::Unreadable { strokes } => {
                write!(f, "{strokes} mark(s) inside that nothing can name yet")
            }
            Warning::Unbalanced { heading, lean } => write!(
                f,
                "unbalanced: leans {lean:.2} toward {:.0}°",
                heading.to_degrees()
            ),
            Warning::Asymmetric => write!(f, "asymmetric — legal but unstable"),
            Warning::NotReversible(id) => {
                write!(f, "'{}' cannot be reversed; flag ignored", id.as_str())
            }
            Warning::RegionIndeterminate => write!(f, "region signs point nowhere in particular"),
            Warning::IntensityUnmeasured => write!(f, "sigil size unmeasured; intensity assumed"),
            Warning::OuterRingOpen(id) => write!(f, "held shut by open outer ring #{}", id.0),
            Warning::Amplified { copies, factor } => {
                write!(f, "amplified ×{factor:.2} by {copies} linked twin(s)")
            }
            Warning::Cancelled(id) => write!(f, "cancelled by reversed twin #{}", id.0),
            Warning::DanglingLink(id) => write!(f, "links to #{}, which is not here", id.0),
            Warning::Unsatisfied { sign, needs } => write!(
                f,
                "'{}' does nothing without '{}'",
                sign.as_str(),
                needs.as_str()
            ),
            Warning::UnsatisfiedOneOf { sign, needs } => {
                let list: Vec<&str> = needs.iter().map(|id| id.as_str()).collect();
                write!(f, "'{}' needs one of {:?}", sign.as_str(), list)
            }
            Warning::Unpaired { sign, usually_with } => write!(
                f,
                "'{}' is only ever seen with '{}'",
                sign.as_str(),
                usually_with.as_str()
            ),
            Warning::NestingCycle => write!(f, "nesting leads back to itself"),
        }
    }
}

/// The thresholds compilation needs, and the reason they live in one struct.
///
/// Every field is either an angle or a ratio, which is the same units test
/// [`RingRules`] applies: a pixel is a fact about one screen, a ratio is a
/// statement about magic (§4.2). Anything measured in pixels reached the glyph
/// long before this point.
///
/// **The numbers are ours.** Canon states each tradeoff and gives no
/// thresholds (§2.6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompileRules {
    /// How neat a closed ring must be before it holds. Shared with assembly so
    /// the overlay and the compiler cannot disagree about the same ring.
    pub ring: RingRules,
    /// How far off tangent a sign must point before it counts as pointing in or
    /// out, in radians. Signs within this of tangent are neither.
    pub radial_deadband: f32,
    /// How tightly region signs must agree before they read as one direction,
    /// in radians.
    pub heading_spread: f32,
    /// Slack in the symmetry classification, in radians.
    pub symmetry_tolerance: f32,
    /// How far a seal may lean, as a share of its total power, before it is
    /// worth warning about.
    pub balance_tolerance: f32,
}

impl Default for CompileRules {
    fn default() -> Self {
        CompileRules {
            ring: RingRules::default(),
            // About 11°. A hand aiming a sign at the centre lands well inside
            // this; a sign meant to lie tangent lands well outside it.
            radial_deadband: 0.2,
            // 30°: four signs aimed the same way by hand still read as one
            // direction, while opposed ones never could.
            heading_spread: 0.52,
            symmetry_tolerance: 0.15,
            // A tenth of total power. Below this the drift is smaller than the
            // slop in drawing the signs.
            balance_tolerance: 0.1,
        }
    }
}

/// A compiled seal: the verdict, the numbers behind it, and the warnings.
///
/// Everything here is derived. Nothing is stored that the glyph and the
/// catalogue could not answer again, which is what makes recompiling after
/// every stroke cheap and makes two drawings of the same seal compile
/// identically (§5).
#[derive(Debug, Clone, PartialEq)]
pub struct Spell {
    /// Which glyph this came from, so a warning can be traced back to ink.
    pub glyph: GlyphId,
    /// Whether it is running.
    pub firing: Firing,
    /// What drives the effect.
    pub driver: Driver,
    /// What the driving sigil can do to substance — the conservation rules the
    /// simulation will enforce. `None` for a discharge, which has no substance
    /// to conserve, and for a substitute, whose capabilities the wiki does not
    /// state.
    pub caps: Option<Capabilities>,
    /// How strongly the effect lands, `0..=1`.
    ///
    /// Canon ties this to "the size of a sigil in relation to the ring", so it
    /// is the ratio of [`Glyph::sigil_extent`] to the ring's radius — not the
    /// ring's absolute size, which rule 8 handles separately.
    pub intensity: f32,
    /// The ring's own radius — canon rule 8's absolute size.
    ///
    /// Kept separate from [`Spell::intensity`] because they are two different
    /// canon claims that were being conflated. Intensity is a *ratio* ("the
    /// size of a sigil in relation to the ring"); this is the seal's outright
    /// size ("larger seals are more powerful than smaller ones"). A big seal
    /// with a small sigil is powerful and unfocused; a small seal with a
    /// filling sigil is focused and weak.
    ///
    /// No formula turns this into a number, on purpose — canon gives none, and
    /// the simulation is where a scale finally has to mean something.
    pub scale: f32,
    /// How neatly the ring was drawn, `0..=1`.
    ///
    /// Canon rule 8, and the half of it `firing` throws away: `Fleeting` says
    /// the ring was *too* rough to hold, and says nothing about the difference
    /// between a good ring and a perfect one. "Neatly drawn seals are more
    /// stable and long-lasting than messy ones" is a continuous claim, so the
    /// continuous number has to survive to whoever runs the spell.
    pub quality: f32,
    /// How many effective signs the seal carries.
    ///
    /// Canon reads count *separately* from size: "the amount of signs will
    /// affect the range or quantity of magic generated for a spell", while
    /// size is power (§2.4). Three independent knobs — count, size, tilt — and
    /// [`Balance::power`] only sums the second.
    pub sign_count: usize,
    /// How firmly the spell embeds in a body, from its glaives (§2.1).
    ///
    /// Zero when none are drawn, which is the ordinary case — glaives have been
    /// nearly forgotten since the Day of the Pact. Measured as total glaive
    /// size against the ring, the same way intensity measures the sigil,
    /// because size is the only thing canon gives us to read here.
    ///
    /// Whether it means depth or tenacity is genuinely unknown and stays so.
    pub embedding: f32,
    /// What substance the spell needs and whether it must find it (§3.2).
    ///
    /// `None` for a discharge and for a substitute driver, the same cases that
    /// have no [`Spell::caps`].
    pub demand: Option<Demand>,
    /// Where the spell will actually go (§2.4).
    pub balance: Balance,
    /// Where its power gathers, from where every steering sign *aims*.
    ///
    /// The companion to `balance`, and not derivable from it: four arrows
    /// pointing at the middle sum to zero drift, so `balance` correctly reports
    /// a spell that goes nowhere and cannot say that all four are pushing at
    /// one point. That is the whole difference between a water orb and a
    /// fountain.
    pub focus: Focus,
    /// What its tilt buys and costs (§2.4).
    pub spin: Spin,
    /// Where it manifests, from every region sign together (§2.3).
    pub region: RegionArrangement,
    /// How the signs are arranged (rule 7).
    pub symmetry: Symmetry,
    /// Signs the catalogue recognised and that contribute behaviour, in the
    /// order they were drawn. Decorative signs are excluded — they are drawn,
    /// recognised, and then do nothing.
    pub effective: Vec<SignId>,
    /// Which of those were drawn mirrored, and are therefore doing the opposite
    /// of what they say (rule 6).
    ///
    /// Only signs the catalogue says *can* be reversed appear here. A mirrored
    /// non-directional sign is a drawing with a flag nobody can act on, which
    /// is what [`Warning::NotReversible`] reports.
    pub inverted: Vec<SignId>,
    /// Every effective sign the catalogue says *could* be drawn mirrored,
    /// inverted or not.
    ///
    /// The denominator rule 6 needs. Without it, a seal sharing an upright
    /// reversible sign with another reads as that seal's twin — the two are
    /// only twins if *every* sign that can flip did.
    pub reversible: Vec<SignId>,
    /// How much linked twins multiply this seal, `1.0` when it stands alone
    /// (rule 5). **Ours** — see [`amplification`].
    pub amplification: f32,
    /// Whether a linked twin cancels this seal out entirely (rule 6).
    pub cancelled: bool,
    /// Everything worth saying about it. Never fatal.
    pub warnings: Vec<Warning>,
}

impl Spell {
    /// Whether the spell is producing an effect right now.
    ///
    /// [`Firing::Fleeting`] counts: canon says a ring that is not circular
    /// enough gives a fleeting effect *or* fails outright, so it does something
    /// briefly rather than nothing at all.
    pub fn fires(&self) -> bool {
        matches!(self.firing, Firing::Active | Firing::Fleeting)
    }

    /// Whether this seal is rule 9's explosion — a ring with nothing driving it.
    pub fn is_discharge(&self) -> bool {
        self.driver == Driver::Discharge
    }

    /// What the spell actually delivers, after everything the pad does to it.
    ///
    /// Zero for a seal that is not firing and for one a twin has cancelled;
    /// otherwise intensity scaled by whatever its linked copies add.
    pub fn strength(&self) -> f32 {
        if !self.fires() || self.cancelled {
            return 0.0;
        }
        self.intensity * self.amplification
    }

    /// Whether two seals are the same spell — the test rule 5 amplifies on.
    ///
    /// Driver and sign *kinds*, not geometry. Canon says "several small,
    /// identical seals", and two seals drawn at different sizes in different
    /// places are still the same spell.
    pub fn same_spell(&self, other: &Spell) -> bool {
        self.driver == other.driver && self.effective == other.effective
    }

    /// Whether `other` is this seal drawn mirrored — rule 6's twin.
    ///
    /// The same spell, with every reversible sign flipped the other way. A
    /// partial flip is not a twin: canon says a spell and its reversed twin
    /// "cancel out completely", which only follows if the inversion is total.
    pub fn is_twin_of(&self, other: &Spell) -> bool {
        if !self.same_spell(other) {
            return false;
        }
        // Every reversible sign appears on exactly one side. Nothing may be
        // inverted in both, and nothing upright in both.
        self.reversible == other.reversible
            && !self.reversible.is_empty()
            && self
                .reversible
                .iter()
                .all(|id| self.inverted.contains(id) != other.inverted.contains(id))
    }
}

/// Reads a glyph into the spell it will cast.
///
/// Total: every glyph compiles. See the module docs for why there is no error
/// type.
pub fn compile(glyph: &Glyph, catalog: &Catalog, rules: &CompileRules) -> Spell {
    let mut warnings = Vec::new();

    let driver = resolve_driver(glyph, catalog, &mut warnings);
    let caps = match &driver {
        Driver::Sigil(id) => catalog.capabilities(id),
        // A substitute's capabilities are an honest hole: canon says billow and
        // stability can occupy the centre and never says what they may create.
        Driver::Substitute(_) | Driver::Discharge => None,
    };

    let demand = match &driver {
        Driver::Sigil(id) => catalog.sigil(id).map(|def| Demand {
            substance: def.affects.clone(),
            must_find: !def.caps.create,
            create_cost: def.create_cost_multiplier,
        }),
        Driver::Substitute(_) | Driver::Discharge => None,
    };

    let firing = fire(glyph, rules, &mut warnings);
    let (effective, inverted, reversible) = effective_signs(glyph, catalog, &mut warnings);
    let intensity = intensity(glyph, &driver, &mut warnings);
    let radius = glyph.ring.radius();
    // Glaives are the one mark allowed outside the ring (rule 1's exception),
    // so nothing here filters them by position.
    let embedding = if radius > f32::EPSILON {
        // `+ 0.0` collapses IEEE's negative-zero identity, which `clamp` will
        // not: `(-0.0).clamp(0.0, 1.0)` is still `-0.0`, and it printed as
        // `embed -0.00` on a seal with no glaives at all.
        ((glyph.glaives.iter().map(|g| g.size.abs()).sum::<f32>() + 0.0) / radius).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let directional = |id: &SignId| catalog.is_directional(id);
    let balance = balance(&glyph.signs, directional);
    let spin = spin(&glyph.signs, directional);
    let symmetry = glyph.symmetry(rules.symmetry_tolerance);
    let region = region(glyph, catalog, rules);
    // Where the signs *aim*, as against which way they push. Four arrows at the
    // middle cancel to zero drift — `balance` is right that the spell goes
    // nowhere, and silent about the thing that makes a water orb an orb.
    let focus = focus(&glyph.signs, radius, directional);

    if !balance.is_balanced(rules.balance_tolerance) {
        warnings.push(Warning::Unbalanced {
            heading: balance.heading,
            lean: balance.lean(),
        });
    }
    if symmetry == Symmetry::Asymmetric {
        warnings.push(Warning::Asymmetric);
    }
    if region == RegionArrangement::Indeterminate {
        warnings.push(Warning::RegionIndeterminate);
    }

    Spell {
        glyph: glyph.id,
        firing,
        driver,
        caps,
        demand,
        intensity,
        scale: radius,
        quality: glyph.ring.quality(),
        sign_count: effective.len(),
        embedding,
        balance,
        spin,
        focus,
        region,
        symmetry,
        effective,
        inverted,
        reversible,
        // A glyph compiled on its own has no pad around it. `compile_all` is
        // what knows about twins and links, and it is the only thing that may
        // change these.
        amplification: 1.0,
        cancelled: false,
        warnings,
    }
}

/// Finds what drives the spell, in canon's own order of preference.
///
/// A sigil first, because the vast majority of seals have one. Then a sign
/// allowed to stand in for one (§2.3). Then rule 9's discharge — which is a
/// real spell, not a fallback.
fn resolve_driver(glyph: &Glyph, catalog: &Catalog, warnings: &mut Vec<Warning>) -> Driver {
    if let Some(id) = &glyph.sigil {
        if catalog.sigil(id).is_some() {
            return Driver::Sigil(id.clone());
        }
        // Named but unknown. Reported and then dropped: inventing behaviour for
        // an id nobody has defined is exactly what §2 forbids.
        warnings.push(Warning::UnknownSigil(id.clone()));
    }

    if let Some(sign) = glyph
        .signs
        .iter()
        .find(|sign| catalog.can_substitute_as_sigil(&sign.kind))
    {
        return Driver::Substitute(sign.kind.clone());
    }

    warnings.push(match (glyph.signs.is_empty(), glyph.unnamed) {
        // Rule 9 in its literal form: the ring is the only thing drawn.
        (true, 0) => Warning::BareRing,
        // Ink is there and nobody could read it. Saying "bare ring, this is an
        // explosion" about a seal covered in marks is worse than saying nothing.
        (_, unnamed) if unnamed > 0 => Warning::Unreadable { strokes: unnamed },
        _ => Warning::NoDriver,
    });
    Driver::Discharge
}

/// Applies canon's two gates on the ring, in order.
///
/// Structure before craft, the same order [`crate::assembly::Activation`] uses:
/// how neatly a ring was drawn is not a question worth asking until it is
/// closed, because an open ring is armed however roughly it was inked.
fn fire(glyph: &Glyph, rules: &CompileRules, warnings: &mut Vec<Warning>) -> Firing {
    // Asked first, and before closure, because "closed" and "neat" are
    // meaningless questions about a triangle. `Activation` has asked in this
    // order since M4.4 and the compiler did not ask at all — which is how a
    // fire sigil drawn on its own, with no ring anywhere near it, came to fire
    // as canon rule 9's explosion. Its triangle is a closed loop that fits a
    // circle; it is not a ring, and canon has no spell without one.
    if !glyph.ring.is_simple() {
        warnings.push(Warning::NotARing);
        return Firing::Inert;
    }
    if !glyph.ring.is_closed() {
        warnings.push(Warning::RingOpen);
        return Firing::Inert;
    }
    let quality = glyph.ring.quality();
    if quality < rules.ring.min_quality {
        warnings.push(Warning::RoughRing { quality });
        return Firing::Fleeting;
    }
    Firing::Active
}

/// Whether a sign's companions are on the seal (§2.3 data).
///
/// Three relations, three different strengths, and the difference matters:
///
/// - `requires` — hard. Every one must be present or the sign does nothing.
/// - `requires_one_of` — hard, but any one will do. Enlarge needs *either*
///   selection or diamond, because those are what decide self versus nearby.
/// - `pairs_with` — soft. The wiki has only ever seen the two together, so the
///   sign alone is unrecorded rather than broken. Reported and kept.
///
/// A sign's own kind is ignored when looking for companions, so a requirement
/// naming itself could never be satisfied by itself.
fn satisfied(id: &SignId, glyph: &Glyph, catalog: &Catalog, warnings: &mut Vec<Warning>) -> bool {
    let Some(def) = catalog.sign(id) else {
        return true;
    };
    let present = |want: &SignId| {
        glyph
            .signs
            .iter()
            .any(|other| &other.kind == want && want != id)
    };

    let mut ok = true;
    for want in &def.requires {
        if !present(want) {
            warnings.push(Warning::Unsatisfied {
                sign: id.clone(),
                needs: want.clone(),
            });
            ok = false;
        }
    }
    if !def.requires_one_of.is_empty() && !def.requires_one_of.iter().any(present) {
        warnings.push(Warning::UnsatisfiedOneOf {
            sign: id.clone(),
            needs: def.requires_one_of.clone(),
        });
        ok = false;
    }
    for want in &def.pairs_with {
        if !present(want) {
            warnings.push(Warning::Unpaired {
                sign: id.clone(),
                usually_with: want.clone(),
            });
        }
    }
    ok
}

/// Sorts the signs into the ones that actually do something.
///
/// Three ways a drawn sign contributes nothing, and only one of them is a
/// mistake: the catalogue does not know it (reported), it is decorative
/// (silent — a bird is meant to be inert), or it is reversed when its class
/// forbids reversal (reported, and the flag ignored rather than the sign
/// dropped).
fn effective_signs(
    glyph: &Glyph,
    catalog: &Catalog,
    warnings: &mut Vec<Warning>,
) -> (Vec<SignId>, Vec<SignId>, Vec<SignId>) {
    let mut kept = Vec::with_capacity(glyph.signs.len());
    let mut inverted = Vec::new();
    let mut reversible = Vec::new();
    for sign in &glyph.signs {
        if catalog.sign(&sign.kind).is_none() {
            warnings.push(Warning::UnknownSign(sign.kind.clone()));
            continue;
        }
        // Canon is explicit that a radial sign has no front, so there is no way
        // to make it point inward. The drawing is legal; the flag is not.
        //
        // Only a stated `false` warns. `None` means the wiki does not say, and
        // reporting an unclassified sign as un-reversible would be inventing a
        // rule rather than applying one.
        if sign.reversed && catalog.is_reversible(&sign.kind) == Some(false) {
            warnings.push(Warning::NotReversible(sign.kind.clone()));
        }
        if catalog.is_decorative(&sign.kind) {
            continue;
        }
        if !satisfied(&sign.kind, glyph, catalog, warnings) {
            // Reported and dropped. Canon's own wording for billow is that it
            // "needs collection to gather the material first" — without it
            // there is nothing for the sign to act on, so it contributes
            // nothing rather than acting on nothing.
            continue;
        }
        // Only a sign canon says can be reversed counts as inverted. A flag on
        // one that cannot is already reported and must not go on to make the
        // seal look like somebody's twin.
        if catalog.is_reversible(&sign.kind) == Some(true) {
            reversible.push(sign.kind.clone());
            if sign.reversed {
                inverted.push(sign.kind.clone());
            }
        }
        kept.push(sign.kind.clone());
    }
    (kept, inverted, reversible)
}

/// How strongly the effect lands, as the wiki measures it.
///
/// The ratio of the driver's ink to the ring that holds it. A sigil filling its
/// ring is `1.0`; a small one in a large ring is weak. Rule 1 keeps the driver
/// inside or touching the ring, so the ratio cannot legitimately exceed one and
/// is clamped rather than reported.
///
/// A discharge is the whole ring going off at once, so it is always full
/// strength — there is nothing smaller than the ring for it to be a fraction of.
fn intensity(glyph: &Glyph, driver: &Driver, warnings: &mut Vec<Warning>) -> f32 {
    if *driver == Driver::Discharge {
        return 1.0;
    }
    let radius = glyph.ring.radius();
    if radius <= f32::EPSILON || glyph.sigil_extent <= 0.0 {
        warnings.push(Warning::IntensityUnmeasured);
        return 1.0;
    }
    (glyph.sigil_extent / radius).clamp(0.0, 1.0)
}

/// Runs the collective region analysis over whichever region sign this seal
/// uses.
///
/// Which sign that is comes from the catalogue rather than being named here —
/// only `region` carries an `arrangements` block today, and a second one would
/// be picked up by adding the data and nothing else (§4.4).
fn region(glyph: &Glyph, catalog: &Catalog, rules: &CompileRules) -> RegionArrangement {
    for id in catalog.region_signs() {
        let found = RegionArrangement::classify(
            &glyph.signs,
            id,
            rules.radial_deadband,
            rules.heading_spread,
        );
        if found != RegionArrangement::Absent {
            return found;
        }
    }
    RegionArrangement::Absent
}

/// How much `copies` identical linked seals multiply each other.
///
/// **Ours.** Canon states the effect and gives no number: "when several small,
/// identical seals are linked together, their combined strength will often be
/// more than would be possible for a single large spell that took up the same
/// amount of space" (rule 5).
///
/// So the total must grow *faster* than the count — otherwise linking would be
/// exactly as good as drawing one bigger seal, and canon says it is better. We
/// take each seal's share as `sqrt(n)`, which puts the total at `n^1.5`: two
/// linked seals are worth 2.8, four are worth 8. Superlinear, unbounded only in
/// the same way ink is, and marked here rather than buried (§2.6).
pub fn amplification(copies: usize) -> f32 {
    (copies.max(1) as f32).sqrt()
}

/// Compiles a whole pad, with every glyph judged against the others.
///
/// Three canon rules are about glyphs *in relation*, and no amount of looking
/// at one seal can answer them:
///
/// - **rule 4** — the outermost ring gates every ring inside it, even one with
///   no gap of its own;
/// - **rule 5** — identical linked seals amplify each other;
/// - **rule 6** — a seal and its reversed twin cancel completely.
///
/// [`compile`] stays exactly as it was and is still the whole story for one
/// glyph. This runs it per glyph and then applies the three, in that order:
/// gating first, because a seal held shut by an outer ring has nothing to
/// amplify or cancel.
///
/// Ids are matched against the glyphs given. A `parent` or `linked` naming
/// something absent is reported as [`Warning::DanglingLink`] and otherwise
/// ignored — half a pad must still compile, since that is what the pad looks
/// like between strokes.
pub fn compile_all(glyphs: &[Glyph], catalog: &Catalog, rules: &CompileRules) -> Vec<Spell> {
    let mut spells: Vec<Spell> = glyphs
        .iter()
        .map(|glyph| compile(glyph, catalog, rules))
        .collect();

    gate_nesting(glyphs, &mut spells);
    resolve_links(glyphs, &mut spells);
    spells
}

/// Rule 4: an open ring anywhere further out holds everything inside it shut.
///
/// > "In nested glyphs, the inner ring will only activate if the outer ring is
/// > completed, even if there is no gap in the inner ring."
///
/// Walks outward rather than inward, so a chain three deep costs one pass per
/// glyph and needs no ordering of the input. The step cap is what keeps a
/// malformed `parent` chain from spinning — §4.7 has core reporting, never
/// hanging.
fn gate_nesting(glyphs: &[Glyph], spells: &mut [Spell]) {
    let index = |id: GlyphId| glyphs.iter().position(|g| g.id == id);

    for i in 0..glyphs.len() {
        let mut at = glyphs[i].parent;
        let mut steps = 0;
        while let Some(parent) = at {
            steps += 1;
            if steps > glyphs.len() {
                spells[i].warnings.push(Warning::NestingCycle);
                break;
            }
            let Some(outer) = index(parent) else {
                spells[i].warnings.push(Warning::DanglingLink(parent));
                break;
            };
            if !glyphs[outer].ring.is_closed() {
                spells[i].firing = Firing::Inert;
                spells[i].warnings.push(Warning::OuterRingOpen(parent));
                break;
            }
            at = glyphs[outer].parent;
        }
    }
}

/// Rules 5 and 6, which are the same lookup read two ways.
///
/// A drawn line between two glyphs says they interact. Whether that interaction
/// adds or annihilates is decided by the seals themselves: the same spell twice
/// amplifies, the same spell mirrored cancels. Cancellation is checked first,
/// because a cancelled seal has no strength left to multiply.
///
/// Links are treated as mutual even when only one side records them — a drawn
/// line has no direction, and demanding both sides list each other would make
/// the answer depend on which glyph was assembled first (§4.3).
fn resolve_links(glyphs: &[Glyph], spells: &mut [Spell]) {
    let index = |id: GlyphId| glyphs.iter().position(|g| g.id == id);

    let mut partners: Vec<Vec<usize>> = vec![Vec::new(); glyphs.len()];
    for (i, glyph) in glyphs.iter().enumerate() {
        for &id in &glyph.linked {
            match index(id) {
                Some(j) if j != i => {
                    if !partners[i].contains(&j) {
                        partners[i].push(j);
                    }
                    if !partners[j].contains(&i) {
                        partners[j].push(i);
                    }
                }
                Some(_) => {}
                None => spells[i].warnings.push(Warning::DanglingLink(id)),
            }
        }
    }

    let mut cancelled = vec![None; glyphs.len()];
    let mut copies = vec![0usize; glyphs.len()];
    for i in 0..spells.len() {
        for &j in &partners[i] {
            if spells[i].is_twin_of(&spells[j]) {
                cancelled[i] = Some(glyphs[j].id);
            } else if spells[i].same_spell(&spells[j]) {
                copies[i] += 1;
            }
        }
    }

    for (i, spell) in spells.iter_mut().enumerate() {
        if let Some(by) = cancelled[i] {
            spell.cancelled = true;
            spell.warnings.push(Warning::Cancelled(by));
            continue;
        }
        if copies[i] > 0 {
            let factor = amplification(copies[i] + 1);
            spell.amplification = factor;
            spell.warnings.push(Warning::Amplified {
                copies: copies[i],
                factor,
            });
        }
    }
}

#[cfg(test)]
#[path = "tests/compiler.rs"]
mod tests;

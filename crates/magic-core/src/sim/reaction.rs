//! What happens when two element instances meet.
//!
//! **The whole module is ours** (§2.6). Canon has no reactions at all — spells
//! are discrete effects with no mechanism behind them, and the wiki never says
//! that wind and water make a vortex. So the rules live in
//! `the-magic-assets/reactions.ron` rather than in `match` arms (§4.4), and
//! arguing with one costs an edit instead of a recompile.
//!
//! What is *not* ours is what the rules have to obey. §3.2's capability model
//! only means anything if matter is conserved, so:
//!
//! - **Mass.** A reaction's output shares must sum to `1.0`. [`ReactionBook`]
//!   refuses the file otherwise, at load, because a rule that quietly mints mass
//!   would make every conservation property in `sim` a lie.
//! - **Momentum.** Products inherit the momentum of what was consumed, split by
//!   mass. Two parcels meeting head-on leave as one that is going nowhere.
//! - **Heat is accounted, not conserved.** A reaction may release or absorb it —
//!   that is what makes boiling plateau and freezing hold a pond at zero — and
//!   the amount is on the rule where it can be argued with.
//!
//! # Determinism (§4.3)
//!
//! Parcels are bucketed into cells by walking the parcel list in order into a
//! `Vec` of `Vec<usize>` — no map, no hashing. Reactions are tried in file
//! order, cells in index order, and every sum walks a list whose order is fixed.

use serde::Deserialize;

use crate::catalog::CatalogError;

use super::field::Field;
use super::parcel::{Parcel, SubstanceId};
use super::vec2::Vec2;

/// The file layout this module understands.
pub const FORMAT_VERSION: u32 = 1;

/// One rule: what meets what, what comes out, and when.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ReactionDef {
    pub id: String,
    /// Substances that must share a cell. **One** input is a phase change; two
    /// or more is a reaction.
    pub inputs: Vec<String>,
    /// What comes out, and each product's share of the consumed mass. The
    /// shares must total `1.0`.
    pub outputs: Vec<(String, f32)>,
    #[serde(default)]
    pub min_temp: Option<f32>,
    #[serde(default)]
    pub max_temp: Option<f32>,
    /// Degrees released (positive) or absorbed (negative) per unit of mass.
    pub heat: f32,
    /// Share of the available mass converted per second.
    pub rate: f32,
    #[serde(default)]
    pub note: Option<String>,
}

impl ReactionDef {
    /// Whether a cell at this temperature is in the rule's window.
    fn window(&self, temperature: f32) -> bool {
        self.min_temp.is_none_or(|floor| temperature >= floor)
            && self.max_temp.is_none_or(|ceiling| temperature <= ceiling)
    }

    /// A rule with one input changes a substance rather than combining two.
    pub fn is_phase_change(&self) -> bool {
        self.inputs.len() == 1
    }
}

#[derive(Debug, Deserialize)]
struct ReactionFile {
    version: u32,
    reactions: Vec<ReactionDef>,
}

/// Every reaction rule, checked.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReactionBook {
    rules: Vec<ReactionDef>,
}

impl ReactionBook {
    /// Parses the rules and refuses anything that would break conservation.
    ///
    /// No filesystem: the caller hands in the file's contents, exactly as
    /// [`crate::Catalog`] does, so core still compiles to wasm untouched (§4.1).
    pub fn parse(file: &'static str, source: &str) -> Result<ReactionBook, CatalogError> {
        // `implicit_some`, exactly as `catalog.rs` does it, so `min_temp: 100.0`
        // reads the same way here as `variant_of: "fire"` does there. One
        // dialect across the assets, or every file is its own puzzle.
        let parsed: ReactionFile = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(source)
            .map_err(|error| CatalogError::Parse {
                file,
                message: error.to_string(),
            })?;

        if parsed.version != FORMAT_VERSION {
            return Err(CatalogError::Parse {
                file,
                message: format!(
                    "version {} but this build reads version {FORMAT_VERSION}",
                    parsed.version
                ),
            });
        }

        for rule in &parsed.reactions {
            if rule.inputs.is_empty() {
                return Err(CatalogError::Parse {
                    file,
                    message: format!("{:?} has no inputs", rule.id),
                });
            }
            if rule.outputs.is_empty() {
                return Err(CatalogError::Parse {
                    file,
                    message: format!("{:?} has no outputs", rule.id),
                });
            }
            // The check the whole module rests on. A share table that does not
            // total one is a rule that creates or destroys matter, and §3.2
            // stops being enforceable the moment one of those is loaded.
            let total: f32 = rule.outputs.iter().map(|(_, share)| *share).sum();
            if (total - 1.0).abs() > 1e-3 {
                return Err(CatalogError::Parse {
                    file,
                    message: format!(
                        "{:?} output shares total {total}, which would {} mass",
                        rule.id,
                        if total > 1.0 { "create" } else { "destroy" }
                    ),
                });
            }
            if rule.rate <= 0.0 || !rule.rate.is_finite() {
                return Err(CatalogError::Parse {
                    file,
                    message: format!("{:?} has a rate of {}", rule.id, rule.rate),
                });
            }
            if let (Some(floor), Some(ceiling)) = (rule.min_temp, rule.max_temp)
                && floor > ceiling
            {
                return Err(CatalogError::Parse {
                    file,
                    message: format!("{:?} can never fire: {floor} > {ceiling}", rule.id),
                });
            }
        }

        // Duplicate ids are not an error — two rules may legitimately turn the
        // same pair into different things in different temperature windows,
        // which is exactly what `steam_burst` and `smother` do.
        Ok(ReactionBook {
            rules: parsed.reactions,
        })
    }

    pub fn rules(&self) -> &[ReactionDef] {
        &self.rules
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Every substance any rule mentions, sorted and deduplicated.
    ///
    /// Sorted so a caller listing them gets the same list every run (§4.3).
    pub fn substances(&self) -> Vec<String> {
        let mut found: Vec<String> = self
            .rules
            .iter()
            .flat_map(|rule| {
                rule.inputs
                    .iter()
                    .cloned()
                    .chain(rule.outputs.iter().map(|(name, _)| name.clone()))
            })
            .collect();
        found.sort();
        found.dedup();
        found
    }
}

/// What one pass of [`react`] did, for a readout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReactionReport {
    /// Mass converted, in total.
    pub converted: f32,
    /// Which rules fired, how much each turned over, and where. Sorted by rule
    /// order, so the list reads the same as the file.
    pub fired: Vec<Fired>,
    /// Net degrees-of-mass released. Negative means the world got colder.
    pub heat: f32,
}

/// One rule, one tick: what it turned over and where it happened.
///
/// The position is the mass-weighted mean of the cells the rule fired in, and
/// it exists so a shell can *show* a reaction. Without it the only honest thing
/// a renderer could do is print a name in a corner, which is not what a person
/// watching water hit fire is looking for.
#[derive(Debug, Clone, PartialEq)]
pub struct Fired {
    pub rule: String,
    pub mass: f32,
    pub at: Vec2,
}

impl ReactionReport {
    pub fn happened(&self) -> bool {
        self.converted > 0.0
    }
}

/// Runs every rule over the field once, for `dt` seconds.
///
/// Cell by cell: substances only meet where they actually are. `Field::settle`
/// must have run — the temperature a rule is tested against is the cell's, not
/// any one parcel's, because a reaction is a thing that happens to a *place*.
pub fn react(field: &mut Field, book: &ReactionBook, dt: f32) -> ReactionReport {
    let mut report = ReactionReport::default();
    if book.is_empty() || !dt.is_finite() || dt <= 0.0 {
        return report;
    }

    for rule in book.rules() {
        let (turned, at) = apply(field, rule, dt);
        if turned > 0.0 {
            report.converted += turned;
            report.heat += turned * rule.heat;
            report.fired.push(Fired {
                rule: rule.id.clone(),
                mass: turned,
                at,
            });
        }
    }

    if report.happened() {
        field.sweep();
        // Products are always *new* parcels, so without this a reacting field
        // gains parcels every tick and never loses any.
        field.coalesce();
        field.settle();
    }
    report
}

/// Which parcels sit in which cell.
///
/// A `Vec` per cell, filled by walking the parcel list in order. Not a map:
/// §4.3 forbids map iteration order in simulation paths, and the cell index is
/// already a perfectly good integer key.
fn buckets(field: &Field) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new(); field.width() * field.height()];
    for (index, parcel) in field.parcels().iter().enumerate() {
        if let Some((col, row)) = field.cell_at(parcel.at) {
            out[row * field.width() + col].push(index);
        }
    }
    out
}

/// One rule, everywhere it applies. Returns the mass it converted.
fn apply(field: &mut Field, rule: &ReactionDef, dt: f32) -> (f32, Vec2) {
    // Cheapest possible early out, and it matters once the room is full of
    // air: bucketing walks every parcel, and most rules have no business
    // being asked about most fields.
    if rule
        .inputs
        .iter()
        .any(|name| !field.parcels().iter().any(|p| p.substance.as_str() == name))
    {
        return (0.0, Vec2::ZERO);
    }

    let cells = buckets(field);
    let mut converted = 0.0;
    // Where it happened, weighted by how much happened there.
    let mut placed = Vec2::ZERO;
    let mut spawned: Vec<Parcel> = Vec::new();

    for (slot, members) in cells.iter().enumerate() {
        if members.is_empty() {
            continue;
        }
        let col = slot % field.width();
        let row = slot / field.width();
        let Some(cell) = field.cell_by(col, row) else {
            continue;
        };
        if !rule.window(cell.temperature) {
            continue;
        }

        // How much of each input is here. The limiting one sets the pace, so
        // one drop of water cannot consume a bonfire.
        let mut available = f32::INFINITY;
        for name in &rule.inputs {
            // Compared as `&str`. The first version built a `SubstanceId` —
            // and therefore a `String` — per input, per cell, per rule, per
            // tick. With a room full of air that is millions of allocations a
            // second for an equality test.
            let here: f32 = members
                .iter()
                .map(|&i| &field.parcels()[i])
                .filter(|parcel| parcel.substance.as_str() == name)
                .map(|parcel| parcel.mass)
                .sum();
            available = available.min(here);
        }
        if !available.is_finite() || available <= 0.0 {
            continue;
        }

        let take = (available * rule.rate * dt).min(available);
        if take <= 0.0 {
            continue;
        }

        // Take `take` of *each* input, so the products carry the sum. Momentum
        // rides along with the mass — two parcels meeting head-on leave as one
        // going nowhere, which is the whole reason to do it this way.
        let mut pooled_mass = 0.0;
        let mut pooled_momentum = Vec2::ZERO;
        let mut pooled_heat = 0.0;

        for name in &rule.inputs {
            let mut left = take;
            let indices: Vec<usize> = members
                .iter()
                .copied()
                .filter(|&i| field.parcels()[i].substance.as_str() == name)
                .collect();
            for index in indices {
                if left <= 0.0 {
                    break;
                }
                let parcel = &mut field.parcels_mut()[index];
                let bite = parcel.mass.min(left);
                pooled_mass += bite;
                pooled_momentum += parcel.velocity * bite;
                pooled_heat += parcel.temperature * bite;
                parcel.mass -= bite;
                left -= bite;
            }
        }

        if pooled_mass <= 0.0 {
            continue;
        }

        let velocity = pooled_momentum * (1.0 / pooled_mass);
        // Degrees per unit mass, plus whatever the rule releases or absorbs.
        let temperature = pooled_heat / pooled_mass + rule.heat * 0.01;
        let at = field.cell_center(col, row);

        for (name, share) in &rule.outputs {
            let mass = pooled_mass * share;
            if mass <= 0.0 {
                continue;
            }
            let mut product = Parcel::new(SubstanceId::new(name.clone()), at, mass);
            product.velocity = velocity;
            product.temperature = temperature;
            spawned.push(product);
        }
        converted += pooled_mass;
        placed += at * pooled_mass;
    }

    for parcel in spawned {
        field.add(parcel);
    }
    let at = if converted > 0.0 {
        placed * (1.0 / converted)
    } else {
        Vec2::ZERO
    };
    (converted, at)
}

#[cfg(test)]
#[path = "../tests/reaction.rs"]
mod tests;

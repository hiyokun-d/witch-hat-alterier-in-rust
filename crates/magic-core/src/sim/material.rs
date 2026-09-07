//! What each substance *is*, and the one formula that makes fire feel like fire.
//!
//! Before this module every parcel fell at the same rate under the same drag,
//! so water and flame were the same simulation wearing different colours. The
//! fix is not a colour and it is not a special case per substance: it is one
//! piece of real physics applied to a density that varies with temperature.
//!
//! # Why fire rises
//!
//! Archimedes: a body in a fluid feels an upward force equal to the weight of
//! the fluid it displaces. Net acceleration is
//!
//! ```text
//! a = g * (1 - rho_fluid / rho_body)
//! ```
//!
//! Heavier than air, the bracket is near `1` and the thing falls at `g`.
//! *Lighter* than air, the bracket goes negative and it climbs.
//!
//! Fire is not a substance with a low density written on it. Fire is **hot
//! gas**, and gas thins as it heats, by the ideal gas law at constant pressure:
//!
//! ```text
//! rho(T) = rho_ref * T_ref / T        (both temperatures absolute)
//! ```
//!
//! Flame at 800 °C is `1073 K` against a room's `293 K`, so it is roughly a
//! third of the room's density, and `1 - 1.2/0.33` is about `-2.6`. Fire climbs
//! at two and a half gravities and slows as it cools, with no rule anywhere
//! saying "flame goes up". That is the whole point of doing it this way: the
//! behaviour is a consequence, so a flame that cools *stops* rising on its own.
//!
//! Steam is the other half of the argument. It is lighter than air even when it
//! has cooled to room temperature, because a water molecule is lighter than the
//! nitrogen it displaces — so steam keeps rising after it stops being hot, and
//! that falls out of `density: 0.6` rather than out of a rule.
//!
//! # What the phases are for
//!
//! Buoyancy alone still leaves water behaving like heavy air. The difference
//! between a liquid and a gas is that a liquid's parts *stay together*, so each
//! phase carries a cohesion: how hard a parcel pulls toward the mean flow of the
//! cell it is in. Water moves as a body and pools; flame disperses.

use serde::Deserialize;

use crate::catalog::CatalogError;

/// The file layout this module understands.
pub const FORMAT_VERSION: u32 = 1;

/// Absolute zero, in the degrees everything else here is measured in.
///
/// The ideal gas law needs absolute temperature, and the rest of `sim` works in
/// Celsius because `AMBIENT` does.
pub const ABSOLUTE_ZERO: f32 = -273.15;

/// How a substance holds itself together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Phase {
    /// Thins as it heats, and that is what makes it rise.
    Gas,
    /// Nearly incompressible, and sticks to itself.
    Liquid,
    /// Falls, lands, and stops.
    Solid,
    /// Piles like a solid, flows like a liquid. Sand.
    Granular,
    /// Neither falls nor floats. Light has no business having a weight.
    Radiant,
}

impl Phase {
    /// Whether the ideal gas law applies. Only a gas thins when it heats.
    pub fn expands(self) -> bool {
        matches!(self, Phase::Gas)
    }

    /// Whether gravity and buoyancy act at all.
    pub fn falls(self) -> bool {
        !matches!(self, Phase::Radiant)
    }

    /// Whether a slow parcel resting on the floor should stay put.
    pub fn settles(self) -> bool {
        matches!(self, Phase::Solid | Phase::Granular)
    }
}

/// One substance, exactly as `materials.ron` records it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MaterialDef {
    pub id: String,
    /// Mass per unit volume at the book's reference temperature.
    pub density: f32,
    pub phase: Phase,
    /// Speed shed per second.
    pub drag: f32,
    /// How hard a parcel matches its cell's mean flow, per second.
    pub cohesion: f32,
    /// Deterministic swirl, in px/s², scaled by how hot the parcel is.
    #[serde(default)]
    pub turbulence: f32,
    /// How firmly a resting `Solid` or `Granular` parcel is held, `0..=1`.
    #[serde(default)]
    pub friction: f32,
    /// How hard the substance resists being crowded, per second.
    ///
    /// The piece that makes a liquid *pool* rather than behave like heavy air.
    /// Buoyancy and cohesion between them give water something that falls and
    /// stays together, and neither of them stops it all collapsing into one
    /// cell — a liquid is nearly incompressible, and without a term saying so
    /// there is no surface, no level, and no heap.
    #[serde(default)]
    pub stiffness: f32,
    /// Rendering only, `0..=1`. Core never reads it; the shell does.
    #[serde(default)]
    pub glow: f32,
    /// The temperature at which a *prop* made of this substance catches fire.
    ///
    /// `None` for anything that does not burn, which is most of the book — and
    /// the absence is the point, because it is what makes a stone plinth a
    /// thing you can safely stand a fire on. Read only by
    /// [`super::prop::step_props`]; a parcel of wood does not burn, since a
    /// parcel is a fluid and combustion here is a property of a body.
    #[serde(default)]
    pub ignites_at: Option<f32>,
    #[serde(default)]
    pub note: Option<String>,
}

impl MaterialDef {
    /// Density at `temperature`, in the same degrees as [`super::AMBIENT`].
    ///
    /// The ideal gas law at constant pressure, for gases only. A liquid or a
    /// solid does expand with heat, but by a fraction of a percent over the
    /// range anything here reaches — modelling it would be noise dressed as
    /// rigour.
    pub fn density_at(&self, temperature: f32, reference: f32) -> f32 {
        if !self.phase.expands() {
            return self.density;
        }
        let absolute = temperature - ABSOLUTE_ZERO;
        let reference_absolute = reference - ABSOLUTE_ZERO;
        if absolute <= 1.0 || !absolute.is_finite() {
            // Below absolute zero is not a state, it is a bug upstream. Report
            // the cold-limit density rather than dividing by something silly.
            return self.density * reference_absolute;
        }
        self.density * reference_absolute / absolute
    }
}

#[derive(Debug, Deserialize)]
struct MaterialFile {
    version: u32,
    ambient_density: f32,
    reference_temp: f32,
    materials: Vec<MaterialDef>,
}

/// Every substance's physical properties, and the medium they sit in.
#[derive(Debug, Clone, PartialEq)]
pub struct Materials {
    entries: Vec<MaterialDef>,
    /// Density of the medium everything is buoyant against.
    pub ambient_density: f32,
    /// The temperature `density` is quoted at.
    pub reference_temp: f32,
    /// What an unlisted substance is treated as.
    fallback: MaterialDef,
}

impl Default for Materials {
    fn default() -> Self {
        Materials {
            entries: Vec::new(),
            ambient_density: 1.2,
            reference_temp: 20.0,
            fallback: Materials::fallback(),
        }
    }
}

impl Materials {
    /// What a substance nobody described behaves like.
    ///
    /// A middling liquid, deliberately: it falls, it holds together a little,
    /// and it is obviously *something*. Returning nothing would mean a
    /// substance added to `reactions.ron` silently stopped moving, which is the
    /// hardest kind of bug to see.
    fn fallback() -> MaterialDef {
        MaterialDef {
            id: "unknown".to_string(),
            density: 900.0,
            phase: Phase::Liquid,
            drag: 1.2,
            cohesion: 2.0,
            turbulence: 0.0,
            friction: 0.0,
            stiffness: 0.0,
            glow: 0.0,
            ignites_at: None,
            note: None,
        }
    }

    /// Parses the file. No filesystem — the caller hands in the contents (§4.1).
    pub fn parse(file: &'static str, source: &str) -> Result<Materials, CatalogError> {
        let parsed: MaterialFile = ron::Options::default()
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
        if !parsed.ambient_density.is_finite() || parsed.ambient_density <= 0.0 {
            return Err(CatalogError::Parse {
                file,
                message: format!("ambient density is {}", parsed.ambient_density),
            });
        }

        for entry in &parsed.materials {
            // A zero density would divide by zero in the buoyancy term, and it
            // is only meaningful for Radiant, which never gets there.
            if entry.phase.falls() && (!entry.density.is_finite() || entry.density <= 0.0) {
                return Err(CatalogError::Parse {
                    file,
                    message: format!("{:?} has density {}", entry.id, entry.density),
                });
            }
            if parsed
                .materials
                .iter()
                .filter(|other| other.id == entry.id)
                .count()
                > 1
            {
                return Err(CatalogError::DuplicateId {
                    file,
                    id: entry.id.clone(),
                });
            }
        }

        Ok(Materials {
            entries: parsed.materials,
            ambient_density: parsed.ambient_density,
            reference_temp: parsed.reference_temp,
            fallback: Materials::fallback(),
        })
    }

    /// The named substance, or the fallback. Never `None`: every parcel has to
    /// behave like *something*.
    pub fn get(&self, id: &str) -> &MaterialDef {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .unwrap_or(&self.fallback)
    }

    /// Whether the book actually describes this substance.
    pub fn describes(&self, id: &str) -> bool {
        self.entries.iter().any(|entry| entry.id == id)
    }

    pub fn all(&self) -> &[MaterialDef] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Net acceleration multiplier for a substance at a temperature.
    ///
    /// `1.0` falls at full gravity, `0.0` hangs, negative climbs. This is the
    /// Archimedes term and nothing else; the caller multiplies by `g`.
    pub fn buoyancy(&self, id: &str, temperature: f32) -> f32 {
        let material = self.get(id);
        if !material.phase.falls() {
            return 0.0;
        }
        let density = material.density_at(temperature, self.reference_temp);
        if !density.is_finite() || density <= 0.0 {
            return 1.0;
        }
        // Clamped because a very hot, very light gas would otherwise accelerate
        // upward faster than anything on screen can be read. **Ours**: the
        // physics has no such limit, the display does.
        (1.0 - self.ambient_density / density).clamp(-6.0, 1.0)
    }
}

#[cfg(test)]
#[path = "../tests/material.rs"]
mod tests;

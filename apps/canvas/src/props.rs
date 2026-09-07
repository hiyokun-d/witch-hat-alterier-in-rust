//! Drawing the things on the paper, and putting them there.
//!
//! Core decides what a prop *is* and what happens to it (`sim::prop`); this
//! decides what it looks like, which is the shell's half of §4.2. Deleting this
//! file would leave the simulation running exactly as it does now, with nothing
//! on screen to show for it.
//!
//! # Why props are not part of the debug overlay
//!
//! They are not a measurement. A plank on the paper is world *content* — the
//! thing a spell is aimed at — and hiding it behind F1 would mean turning the
//! overlay off left you casting at an empty page. The overlay shows you what
//! the engine measured; this shows you what is there.

use bevy::prelude::*;
use magic_core::sim::{Prop, SubstanceId, scatter};

use crate::PaperShape;
use crate::sim::Simulation;

/// Under the ink, over the paper. A prop is a thing you draw *on*, so a seal
/// laid across it has to stay readable.
const PROP_Z: f32 = -5.0;

/// How many props `kindle` scatters at a time.
const HANDFUL: usize = 6;

/// What `kindle` can scatter, in the order the button steps through.
///
/// Every one of them is a substance `materials.ron` already describes, and the
/// list is ordered so the first press gives the case that demonstrates the most:
/// wood catches, burns through, and can be soaked before it does.
pub const KINDLING: &[&str] = &["wood", "cloth", "stone", "ice", "sand"];

/// One line on what a kindling substance is *for*, for the hint line.
///
/// The interesting fact about each is what it refuses to do, because that is
/// what the capability model (§3.2) and `ignites_at` are actually saying.
pub fn describe_kindling(substance: &str) -> &'static str {
    match substance {
        "wood" => "catches at 280 deg, burns through, wet wood will not light",
        "cloth" => "catches at 180 deg and soaks fast - the clearest test of water",
        "stone" => "never burns however hot it gets, only heats and is shoved",
        "ice" => "does not burn; melts through the reaction rules instead",
        "sand" => "does not burn; blows about more readily than stone",
        _ => "no note",
    }
}

/// What each substance looks like as an object, before heat and water get to
/// it.
///
/// A `match` on a name in a shell file, deliberately — what colour wood is on
/// this screen is not a fact about magic, and an unknown substance gets a plain
/// grey rather than a guess so adding one to `materials.ron` shows up as
/// something visible rather than as nothing.
fn prop_color(substance: &str) -> Color {
    match substance {
        "wood" => Color::srgb_u8(0x8B, 0x5E, 0x34),
        "cloth" => Color::srgb_u8(0xC4, 0xA8, 0x7A),
        "stone" => Color::srgb_u8(0x77, 0x74, 0x6E),
        "ice" => Color::srgb_u8(0xBF, 0xDD, 0xE8),
        "sand" => Color::srgb_u8(0xC9, 0xB0, 0x7A),
        "soil" => Color::srgb_u8(0x5C, 0x46, 0x30),
        "crystal" => Color::srgb_u8(0xA9, 0xC6, 0xD8),
        _ => Color::srgb_u8(0x8A, 0x84, 0x7C),
    }
}

/// What a prop looks like right now.
///
/// Four facts have to be readable at a glance and they are deliberately given
/// four *different* channels, because a single brightness ramp made a wet plank
/// and a cold one identical:
///
/// - **wet** darkens and cools the hue, the way real timber does;
/// - **burning** drives it toward ember orange, brightest at the moment it
///   catches;
/// - **lit** lifts it toward white, which is the only thing light does to an
///   object that is not already on fire;
/// - **spent** fades it out as integrity falls, so a plank visibly thins.
fn look(prop: &Prop) -> Color {
    let base = prop_color(prop.substance.as_str());
    let wet = base.mix(&Color::srgb(0.16, 0.20, 0.28), prop.wetness * 0.55);

    let heat = ((prop.temperature - 20.0) / 700.0).clamp(0.0, 1.0);
    let ember = Color::srgb(1.0, 0.45, 0.10);
    let hot = wet.mix(
        &ember,
        if prop.burning {
            0.35 + heat * 0.5
        } else {
            heat * 0.35
        },
    );

    let lit = hot.mix(&Color::WHITE, prop.lit * 0.45);
    // Integrity is the alpha, floored: a prop about to go should still be
    // visible enough to see it go.
    lit.with_alpha(0.35 + prop.integrity.clamp(0.0, 1.0) * 0.65)
}

/// One sprite in the pool, by slot.
#[derive(Component)]
struct PropSprite(usize);

/// Scatters a handful of props across the paper.
///
/// The seed is the tick count, so pressing it twice lays out two different
/// boards while each one stays reproducible — §4.3 forbids an RNG, and this is
/// the honest way to get variety without one.
pub fn kindle(
    world: &mut Simulation,
    shape: PaperShape,
    window: &Window,
    substance: &str,
) -> String {
    let extent = shape.extent(window);
    // Inside the sheet, not merely inside the window: a prop off the paper is
    // one no seal can be drawn around.
    let low = magic_core::Vec2::new(-extent.x * 0.7, -extent.y * 0.7);
    let high = magic_core::Vec2::new(extent.x * 0.7, extent.y * 0.7);
    let seed = (world.sim.ticks as u32).wrapping_add(world.sim.props.len() as u32 * 31 + 1);

    let fresh = scatter(HANDFUL, low, high, SubstanceId::new(substance), seed);
    let n = fresh.len();
    world.sim.props.extend(fresh);
    format!(
        "scattered {n} {substance} - {} on the paper now",
        world.sim.props.len()
    )
}

/// Draws every prop as a filled rectangle, reusing a pool of sprites.
///
/// The same pooling `debug.rs` uses for parcels and for exactly the same
/// reason: the count changes as props burn away, and a pool that churns
/// entities is a pool that stutters.
fn draw_props(
    mut commands: Commands,
    world: Option<Res<Simulation>>,
    mut sprites: Query<(&PropSprite, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let props: &[Prop] = match world.as_ref() {
        Some(world) => &world.sim.props,
        None => &[],
    };

    let mut seen = 0usize;
    for (slot, mut sprite, mut transform, mut visible) in &mut sprites {
        seen = seen.max(slot.0 + 1);
        match props.get(slot.0) {
            Some(prop) => {
                sprite.color = look(prop);
                // Width and height shrink a little as it is consumed, so a
                // half-burnt plank is visibly half a plank rather than a faint
                // whole one.
                let left = 0.55 + prop.integrity.clamp(0.0, 1.0) * 0.45;
                sprite.custom_size = Some(Vec2::new(
                    prop.half.x * 2.0 * left,
                    prop.half.y * 2.0 * left,
                ));
                transform.translation = Vec3::new(prop.at.x, prop.at.y, PROP_Z);
                *visible = Visibility::Inherited;
            }
            None => *visible = Visibility::Hidden,
        }
    }

    for slot in seen..props.len() {
        commands.spawn((
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, PROP_Z),
            PropSprite(slot),
        ));
    }
}

/// Adds the props. Deleting this line leaves the simulation running with
/// nothing on the paper to act on.
pub struct PropsPlugin;

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_props);
    }
}

#[cfg(test)]
#[path = "tests/props.rs"]
mod tests;

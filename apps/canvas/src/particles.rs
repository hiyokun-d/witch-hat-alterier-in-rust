//! Motes: the visual half of things that *happen*.
//!
//! `sim::event` says what occurred; this decides what that looks like. The
//! division is §4.2's, and it is load-bearing here rather than tidy-minded:
//! a blast is over in one tick, so by the time a renderer looks at *state* the
//! only trace left is some parcels moving outward. A moment needs its own
//! representation or it cannot be seen at all.
//!
//! # Determinism (§4.3), and why this file is exempt
//!
//! Motes run on the frame clock, not the fixed step, and their scatter comes
//! from a hash of a counter. Neither is a violation, because **nothing here is
//! ever read back**: no mote touches a parcel, a prop, a field or a spell. Turn
//! the whole module off and every number the simulation reports is identical.
//! That is the test to apply to anything added here — if a mote could change an
//! outcome, it belongs in core instead.
//!
//! # Prepared, not merely used
//!
//! Every substance in `materials.ron` and every variant of `Event` has an entry
//! below, including the ones nothing emits yet. A look table with holes in it
//! fails silently — the effect simply does not appear and nobody can tell
//! whether that is a missing rule or a missing colour. With the table complete,
//! adding a reaction or a sigil means the particles are already waiting.

use bevy::prelude::*;
use magic_core::sim::Event;

use crate::sim::Simulation;

/// Above the parcels, below the panel. A mote is the loudest thing on screen
/// for the moment it exists, and it should not be buried under a dot.
const MOTE_Z: f32 = 6.0;

/// The most motes alive at once.
///
/// A cap rather than a budget: a burst that would exceed it is trimmed, and the
/// oldest motes are the ones that go. Without it a chain of reactions can ask
/// for thousands in a tick and the frame time is what pays.
const MAX_MOTES: usize = 900;

/// What each substance looks like on this screen.
///
/// A `match` on a name in a shell file, deliberately (§4.2): what colour water
/// is here is not a fact about magic. An unknown substance gets a plain grey
/// rather than a guess, so adding one to `materials.ron` shows up as something
/// visible rather than as nothing.
///
/// Lives here rather than in `debug.rs` because §0 keeps that file deletable,
/// and the colours must survive it.
pub fn substance_color(name: &str) -> Color {
    match name {
        "water" => Color::srgb(0.18, 0.45, 0.85),
        "steam" => Color::srgb(0.82, 0.86, 0.92),
        "ice" => Color::srgb(0.55, 0.85, 0.95),
        "flame" => Color::srgb(0.95, 0.35, 0.10),
        "heat" => Color::srgb(0.90, 0.55, 0.20),
        "light" => Color::srgb(0.98, 0.92, 0.55),
        "smoke" => Color::srgb(0.42, 0.40, 0.40),
        "air" => Color::srgb(0.62, 0.82, 0.72),
        "stone" => Color::srgb(0.48, 0.44, 0.40),
        "sand" => Color::srgb(0.80, 0.68, 0.42),
        "soil" => Color::srgb(0.38, 0.28, 0.20),
        "wood" => Color::srgb(0.55, 0.38, 0.22),
        "cloth" => Color::srgb(0.77, 0.66, 0.48),
        "crystal" => Color::srgb(0.70, 0.60, 0.92),
        "electricity" => Color::srgb(0.65, 0.55, 0.98),
        "sound" => Color::srgb(0.85, 0.70, 0.85),
        _ => Color::srgb(0.60, 0.58, 0.55),
    }
}

/// One speck of light with somewhere to be.
#[derive(Debug, Clone, Copy)]
struct Mote {
    at: Vec2,
    velocity: Vec2,
    /// Seconds left, counting down.
    life: f32,
    /// What it started with, so fade is a fraction rather than a guess.
    span: f32,
    size: f32,
    color: Color,
    /// Upward drift per second. Positive for anything that behaves like smoke.
    rise: f32,
    /// Speed shed per second.
    drag: f32,
}

/// The shape of one burst, before it is scattered into motes.
struct Burst {
    count: usize,
    /// Outward speed. Negative draws the motes *inward* to the centre instead,
    /// which is how gathering is told apart from creating.
    speed: f32,
    /// How much the speed varies, `0..=1`.
    jitter: f32,
    life: f32,
    size: f32,
    color: Color,
    rise: f32,
    drag: f32,
    /// Where the motes start, as a radius about the event.
    ring: f32,
}

/// Every mote alive, and the counter their scatter is hashed from.
#[derive(Resource, Default)]
pub struct Motes {
    live: Vec<Mote>,
    seed: u32,
}

impl Motes {
    /// Adds a burst about a point.
    fn scatter(&mut self, at: Vec2, burst: Burst) {
        for index in 0..burst.count {
            if self.live.len() >= MAX_MOTES {
                // Oldest first: the newest burst is the one being watched.
                self.live.remove(0);
            }
            self.seed = self.seed.wrapping_add(1);
            let spin = hash(self.seed, 1) * std::f32::consts::TAU;
            let (sin, cos) = spin.sin_cos();
            let heading = Vec2::new(cos, sin);
            let vary = 1.0 - burst.jitter * hash(self.seed, 2);
            let start = at + heading * (burst.ring * (0.4 + 0.6 * hash(self.seed, 3)));

            // A negative speed means the mote falls toward the middle, so it is
            // launched from the ring and aimed back at it.
            let velocity = heading * (burst.speed * vary);
            self.live.push(Mote {
                at: if burst.speed < 0.0 {
                    start
                } else {
                    at + heading * burst.ring * 0.2
                },
                velocity,
                life: burst.life * (0.7 + 0.6 * hash(self.seed, 4)),
                span: burst.life,
                size: burst.size * (0.7 + 0.6 * hash(self.seed, 5)),
                color: burst.color,
                rise: burst.rise,
                drag: burst.drag,
            });
            let _ = index;
        }
    }
}

/// The same Wang hash `sim::step::swirl` uses, to `0..=1`.
///
/// A hash rather than an RNG even here, where determinism is not required, so
/// there is exactly one way this project makes a number look arbitrary.
fn hash(seed: u32, salt: u32) -> f32 {
    let mut h = seed
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(374_761_393));
    h ^= h >> 15;
    h = h.wrapping_mul(2_246_822_519);
    h ^= h >> 13;
    h as f32 / u32::MAX as f32
}

/// What each kind of event looks like.
///
/// Every variant has an entry. Where two would look the same they are given
/// different *motion* rather than different colour — creating blooms outward
/// and gathering falls inward, which is the capability model (§3.2) made
/// visible: you can see whether a spell made its substance or took it.
fn burst_for(event: &Event) -> Burst {
    match event {
        Event::Blast { strength, .. } => Burst {
            count: (18.0 + strength * 22.0).min(60.0) as usize,
            speed: 220.0 + strength * 160.0,
            jitter: 0.6,
            life: 0.45,
            size: 7.0,
            color: Color::srgb(1.0, 0.78, 0.35),
            rise: 40.0,
            drag: 2.6,
            ring: 6.0,
        },
        Event::Summon {
            substance,
            mass,
            created: true,
            ..
        } => Burst {
            count: (10.0 + mass * 6.0).min(40.0) as usize,
            speed: 90.0,
            jitter: 0.7,
            life: 0.7,
            size: 6.0,
            color: substance_color(substance.as_str()),
            rise: 10.0,
            drag: 2.0,
            ring: 8.0,
        },
        Event::Summon {
            substance, mass, ..
        } => Burst {
            // Inward: this substance was *found*, not made, so it arrives from
            // the room rather than out of the middle.
            count: (10.0 + mass * 6.0).min(40.0) as usize,
            speed: -150.0,
            jitter: 0.5,
            life: 0.6,
            size: 5.0,
            color: substance_color(substance.as_str()),
            rise: 0.0,
            drag: 1.2,
            ring: 70.0,
        },
        Event::Refused { substance, .. } => Burst {
            // §3.2 biting: the spell wanted something the room did not have.
            // Small, grey, and falling — a fizzle has to look like a failure.
            count: 9,
            speed: 40.0,
            jitter: 0.8,
            life: 0.5,
            size: 4.0,
            color: substance_color(substance.as_str()).mix(&Color::srgb(0.3, 0.3, 0.3), 0.7),
            rise: -60.0,
            drag: 3.0,
            ring: 10.0,
        },
        Event::Ignite { .. } => Burst {
            count: 16,
            speed: 70.0,
            jitter: 0.7,
            life: 0.8,
            size: 5.0,
            color: Color::srgb(1.0, 0.62, 0.18),
            rise: 130.0,
            drag: 1.6,
            ring: 12.0,
        },
        Event::Douse { .. } => Burst {
            count: 20,
            speed: 55.0,
            jitter: 0.6,
            life: 1.1,
            size: 9.0,
            color: Color::srgb(0.90, 0.93, 0.96),
            rise: 90.0,
            drag: 1.4,
            ring: 14.0,
        },
        Event::Spent { substance, .. } => Burst {
            count: 14,
            speed: 60.0,
            jitter: 0.7,
            life: 1.0,
            size: 5.0,
            color: substance_color(substance.as_str()).mix(&Color::BLACK, 0.55),
            rise: -40.0,
            drag: 2.2,
            ring: 12.0,
        },
        Event::Flash { strength, .. } => Burst {
            count: (24.0 + strength * 18.0).min(56.0) as usize,
            speed: 300.0,
            jitter: 0.3,
            life: 0.35,
            size: 8.0,
            color: Color::srgb(1.0, 0.98, 0.85),
            rise: 0.0,
            drag: 3.4,
            ring: 4.0,
        },
        Event::Reacted { product, mass, .. } => Burst {
            count: (6.0 + mass * 10.0).min(30.0) as usize,
            speed: 70.0,
            jitter: 0.8,
            life: 0.6,
            size: 5.0,
            color: substance_color(product.as_str()),
            rise: 30.0,
            drag: 2.0,
            ring: 10.0,
        },
    }
}

/// Turns the events waiting in the simulation into motes.
///
/// **Drains** rather than reads, and that is the only write this module makes.
/// A queue read without draining would re-burst every frame the simulation is
/// paused on, and `Sim::step` cannot clear it for us: a fixed step can run
/// twice between two frames, so a tick's events would be gone before anything
/// saw them. Draining is what the queue is for, and it changes no outcome —
/// nothing in the simulation reads its own events back.
fn catch_events(world: Option<ResMut<Simulation>>, mut motes: ResMut<Motes>) {
    let Some(mut world) = world else {
        return;
    };
    if world.sim.events.is_empty() {
        return;
    }
    for event in std::mem::take(&mut world.sim.events) {
        let at = event.at();
        motes.scatter(Vec2::new(at.x, at.y), burst_for(&event));
    }
}

/// Moves every mote and ages it out.
fn advance_motes(time: Res<Time>, mut motes: ResMut<Motes>) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for mote in &mut motes.live {
        mote.velocity.y += mote.rise * dt;
        mote.velocity *= 1.0 - (mote.drag * dt).clamp(0.0, 1.0);
        mote.at += mote.velocity * dt;
        mote.life -= dt;
    }
    motes.live.retain(|mote| mote.life > 0.0);
}

/// One sprite in the pool, by slot.
#[derive(Component)]
struct MoteSprite(usize);

/// Draws every mote, reusing a pool of sprites.
fn draw_motes(
    mut commands: Commands,
    motes: Res<Motes>,
    mut sprites: Query<(&MoteSprite, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let mut seen = 0usize;
    for (slot, mut sprite, mut transform, mut visible) in &mut sprites {
        seen = seen.max(slot.0 + 1);
        match motes.live.get(slot.0) {
            Some(mote) => {
                // Fades and shrinks together. Either alone reads as a mote
                // sliding away rather than one going out.
                let left = (mote.life / mote.span.max(f32::EPSILON)).clamp(0.0, 1.0);
                sprite.color = mote.color.with_alpha(left);
                sprite.custom_size = Some(Vec2::splat(mote.size * (0.35 + left * 0.65)));
                transform.translation = Vec3::new(mote.at.x, mote.at.y, MOTE_Z);
                *visible = Visibility::Inherited;
            }
            None => *visible = Visibility::Hidden,
        }
    }

    for slot in seen..motes.live.len() {
        commands.spawn((
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, MOTE_Z),
            MoteSprite(slot),
        ));
    }
}

/// Adds the motes. Deleting this line changes no number the simulation reports.
pub struct MotesPlugin;

impl Plugin for MotesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Motes>()
            .add_systems(Startup, make_glow)
            .add_systems(
                Update,
                ((catch_events, advance_motes, draw_motes).chain(), draw_glow),
            );
    }
}

#[cfg(test)]
#[path = "tests/particles.rs"]
mod tests;

// ── Glow ────────────────────────────────────────────────────────────────────
//
// The other half of what light does. `sim::light_the_room` darkens the desk as
// a light spell takes hold, which is the *absence* of light; this is the light
// itself. Without both, a light spell reads as the room going dim for no reason.
//
// A soft radial sprite rather than a gizmo circle, and that is the same lesson
// the parcels taught: `gizmos.circle_2d` draws a hairline outline, so a "glow"
// made of them is a wire ring on parchment. The texture is generated at startup
// rather than shipped, because a 64px radial falloff is six lines of arithmetic
// and §11 keeps the repository's only binary the typeface.

/// Under everything else, including the ink: a halo is what a thing is seen
/// *by*, so it must never sit on top of the drawing.
const GLOW_Z: f32 = -8.0;

/// The most halos drawn at once.
///
/// Not a tidiness cap — a real one. A field well alight holds hundreds of flame
/// parcels, and a halo is a large *transparent* sprite, so one per parcel is
/// hundreds of full-screen-ish overdraws in a frame. The brightest survive,
/// which is also the honest trim: light adds, so the ones dropped are the ones
/// already inside somebody else's glow.
const MAX_LAMPS: usize = 40;

/// Side of the generated falloff texture, in pixels. Small on purpose — it is
/// scaled up hugely and blurred by that scaling, which is what a glow wants.
const GLOW_TEXTURE: u32 = 64;

/// The soft round falloff every halo is drawn with.
#[derive(Resource)]
struct GlowTexture(Handle<Image>);

/// One halo in the pool.
#[derive(Component)]
struct GlowSprite(usize);

/// Builds the radial falloff once.
fn make_glow(mut images: ResMut<Assets<Image>>, mut commands: Commands) {
    use bevy::asset::RenderAssetUsages;
    use bevy::image::Image;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    let side = GLOW_TEXTURE as usize;
    let mut data = vec![0u8; side * side * 4];
    let half = side as f32 * 0.5;
    for y in 0..side {
        for x in 0..side {
            let dx = (x as f32 + 0.5 - half) / half;
            let dy = (y as f32 + 0.5 - half) / half;
            // Squared falloff, which reads far softer at the edge than linear
            // and is what stops a halo looking like a disc with a rim.
            let fade = (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let alpha = (fade * fade * 255.0) as u8;
            let at = (y * side + x) * 4;
            data[at] = 255;
            data[at + 1] = 255;
            data[at + 2] = 255;
            data[at + 3] = alpha;
        }
    }

    let image = Image::new(
        Extent3d {
            width: GLOW_TEXTURE,
            height: GLOW_TEXTURE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    commands.insert_resource(GlowTexture(images.add(image)));
}

/// One thing worth casting light: where it is, how big, and what colour.
struct Lamp {
    at: Vec2,
    radius: f32,
    color: Color,
    strength: f32,
}

/// Everything in the world that is giving off light this frame.
///
/// Read from the simulation rather than from a list somebody maintains, so a
/// substance that starts glowing because `materials.ron` changed starts casting
/// light too — the same reason `glow` is a material property at all.
fn lamps(world: &Simulation) -> Vec<Lamp> {
    let mut out = Vec::new();
    for parcel in world.sim.field.parcels() {
        let material = world.sim.materials.get(parcel.substance.as_str());
        if material.glow < 0.2 || parcel.mass <= 0.0 {
            continue;
        }
        // Radius from mass, square-rooted: brightness spreads over an area, so
        // twice the mass is not twice the reach.
        let radius = 40.0 + parcel.mass.sqrt() * 90.0 * material.glow;
        out.push(Lamp {
            at: Vec2::new(parcel.at.x, parcel.at.y),
            radius,
            color: substance_color(parcel.substance.as_str()),
            strength: (material.glow * 0.5).min(0.6),
        });
    }
    // A burning prop is a lamp too, and a large one. Otherwise a plank well
    // alight sits in a dark room lighting nothing, which is the one thing
    // everybody knows fire does.
    for prop in &world.sim.props {
        if !prop.burning {
            continue;
        }
        out.push(Lamp {
            at: Vec2::new(prop.at.x, prop.at.y),
            radius: 90.0 + prop.half.length() * 3.0,
            color: Color::srgb(1.0, 0.62, 0.25),
            strength: 0.45 * prop.integrity.clamp(0.0, 1.0),
        });
    }

    if out.len() > MAX_LAMPS {
        // By how much each actually lights the room — reach times brightness,
        // not either alone. `total_cmp` because a ruined parcel could otherwise
        // put a `NaN` in the comparator and take the sort with it.
        out.sort_by(|a, b| (b.radius * b.strength).total_cmp(&(a.radius * a.strength)));
        out.truncate(MAX_LAMPS);
    }
    out
}

/// Draws a halo for everything giving off light.
fn draw_glow(
    mut commands: Commands,
    world: Option<Res<Simulation>>,
    texture: Option<Res<GlowTexture>>,
    mut sprites: Query<(&GlowSprite, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let (Some(world), Some(texture)) = (world, texture) else {
        return;
    };
    let lit = lamps(&world);

    let mut seen = 0usize;
    for (slot, mut sprite, mut transform, mut visible) in &mut sprites {
        seen = seen.max(slot.0 + 1);
        match lit.get(slot.0) {
            Some(lamp) => {
                sprite.image = texture.0.clone();
                sprite.color = lamp.color.with_alpha(lamp.strength);
                sprite.custom_size = Some(Vec2::splat(lamp.radius * 2.0));
                transform.translation = Vec3::new(lamp.at.x, lamp.at.y, GLOW_Z);
                *visible = Visibility::Inherited;
            }
            None => *visible = Visibility::Hidden,
        }
    }

    for slot in seen..lit.len() {
        commands.spawn((
            Sprite {
                image: texture.0.clone(),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, GLOW_Z),
            GlowSprite(slot),
        ));
    }
}

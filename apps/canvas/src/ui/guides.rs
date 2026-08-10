//! Lines to draw along.
//!
//! The other half of "you should not need to be able to draw". Stamping gives
//! you exact ink; guides help you produce your own — a ring at the stamp's
//! radius to trace over, spokes to keep signs evenly spaced (canon rule 7's
//! radial symmetry), and rings either side of the target so you can see how
//! far off you are while the pen is still moving.
//!
//! Drawn under nothing and over the paper. They are not ink: capture never
//! sees them, the pad never holds them, and clearing the pad leaves them
//! alone.

use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use std::f32::consts::TAU;

use super::ToolState;

/// The ring the stamp would land on.
const TARGET: Color = Color::srgba(0.35, 0.45, 0.62, 0.55);
/// Rings either side of it, for judging how far out a hand-drawn line strayed.
const TOLERANCE: Color = Color::srgba(0.35, 0.45, 0.62, 0.22);
/// Spokes, for spacing signs evenly around the ring.
const SPOKE: Color = Color::srgba(0.35, 0.45, 0.62, 0.28);

/// How far either side of the target ring the tolerance rings sit, as a share
/// of the radius. Five percent is about where a seal stops reading as neat.
const TOLERANCE_BAND: f32 = 0.05;

/// How many spokes. Twelve divides by 2, 3, 4 and 6, so the common radial
/// arrangements all land on one.
const SPOKES: usize = 12;

/// Segments in a guide ring. Enough that it reads as a circle and not a
/// polygon at any size the pad allows.
const RESOLUTION: u32 = 128;

/// Guides get their own group so they can be hairline-thin while ink stays
/// broad — line width is per config group, not per call.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct GuideGizmos;

/// Registers the guide gizmo group and thins it.
pub fn setup(app: &mut App) {
    app.init_gizmo_group::<GuideGizmos>()
        .add_systems(Startup, thin);
}

fn thin(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<GuideGizmos>();
    config.line.width = 1.0;
}

pub fn draw(tools: Res<ToolState>, mut gizmos: Gizmos<GuideGizmos>) {
    if !tools.guides {
        return;
    }

    let center = Isometry2d::from_translation(Vec2::ZERO);
    let radius = tools.stamp_radius;

    gizmos
        .circle_2d(center, radius, TARGET)
        .resolution(RESOLUTION);
    gizmos
        .circle_2d(center, radius * (1.0 + TOLERANCE_BAND), TOLERANCE)
        .resolution(RESOLUTION);
    gizmos
        .circle_2d(center, radius * (1.0 - TOLERANCE_BAND), TOLERANCE)
        .resolution(RESOLUTION);

    // Spokes run from the centre to a little past the ring, so a sign placed
    // just outside it still has a line to sit on.
    for i in 0..SPOKES {
        let a = i as f32 / SPOKES as f32 * TAU;
        let out = Vec2::from_angle(a);
        gizmos.line_2d(Vec2::ZERO, out * radius * 1.15, SPOKE);
    }
}

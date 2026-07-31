use bevy::prelude::*;

static DEBUG_ON_SCREEN: bool = false;

// all of the code will be start here just like C
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.09)))
        .add_systems(Startup, setup)
        .add_systems(Update, cursor_world_position)
        .run();
}

/// Marks the on-screen debug readout so the cursor system can find it again.
/// Temporary scaffolding — it goes away once ink rendering can show the same
/// thing implicitly.
#[derive(Component)]
struct DebugReadout;
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text2d::new("cursor: —"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.45, 0.85, 0.75)),
        // Parked near the top of the window rather than dead center, where the
        // drawing will be.
        Transform::from_xyz(0.0, 300.0, 0.0),
        DebugReadout,
    ));
}

/// Reports where the cursor is in world space, every frame it sits over the
/// window. Good for a backflip later.
///
/// The mouse reports pixels from the top-left with Y growing downward; the
/// world puts its origin at the center with Y growing upward. Everything drawn
/// from here on lives in world space, so the conversion happens once, here.
fn cursor_world_position(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut readout: Single<&mut Text2d, With<DebugReadout>>,
) {
    let (camera, camera_transform) = *camera;

    // Absent whenever the pointer is outside the window — normal, not an error.
    let Some(cursor) = window.cursor_position() else {
        readout.0 = "cursor: off-window".to_string();
        return;
    };

    // Fails only on a degenerate viewport or projection.
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        readout.0 = "cursor: conversion failed".to_string();
        return;
    };

    // TODO: REMOVE THIS CAUSE THIS IS JUST A DEBUG ON SCREEN
    if DEBUG_ON_SCREEN {
        readout.0 = format!(
            "screen {:.0}, {:.0}   →   world {:.0}, {:.0}",
            cursor.x, cursor.y, world_pos.x, world_pos.y
        );
    }
}

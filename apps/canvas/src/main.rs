use bevy::prelude::*;

/// Where the cursor readout goes: `true` draws it in the window, `false` logs
/// it to the terminal. `const` rather than `static` so the dead branch is
/// compiled out entirely.
const DEBUG_ON_SCREEN: bool = false;

// all of the code will be start here just like C
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.09)))
        .add_systems(Startup, setup)
        .add_systems(Update, cursor_world_position)
        .run();
}

/// Spawns the debug overlay. Takes `&mut Commands` rather than owning it — this
/// is a plain helper called from `setup`, not a system Bevy runs on its own.
fn debugger_screen(commands: &mut Commands, font: TextFont) {
    commands.spawn((
        Text2d::new("cursor: —"),
        font,
        TextColor(Color::srgb(0.45, 0.85, 0.75)),
        // Parked near the top of the window rather than dead center, where the
        // drawing will be.
        Transform::from_xyz(0.0, 300.0, 0.0),
        DebugReadout,
    ));
}

/// Marks the on-screen debug readout so the cursor system can find it again.
/// Temporary scaffolding — it goes away once ink rendering can show the same
/// thing implicitly.
#[derive(Component)]
struct DebugReadout;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let font = TextFont {
        font_size: FontSize::Px(18.0),
        ..default()
    };

    //* INFO: every debug element should be generated from here and nothing else
    if DEBUG_ON_SCREEN {
        debugger_screen(&mut commands, font.clone());
    }

    commands.spawn((
        Text2d::new("This app made by HIYO"),
        font,
        TextColor(Color::srgb(0.35, 0.35, 0.40)),
        Transform::from_xyz(0.0, 270.0, 0.0),
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
    readout: Option<Single<&mut Text2d, With<DebugReadout>>>,
) {
    let (camera, camera_transform) = *camera;
    let message = describe_cursor(&window, camera, camera_transform);

    // TODO: REMOVE ALL OF THIS ONCE INK RENDERING SHOWS THE SAME THING
    // `Option` because a bare `Single` that matches nothing makes Bevy skip the
    // whole system — which would take the terminal fallback down with it.
    match readout {
        Some(mut readout) => readout.0 = message,
        None => info!("{message}"),
    }
}

/// Builds the readout line, so the caller only has to decide where it goes.
fn describe_cursor(window: &Window, camera: &Camera, camera_transform: &GlobalTransform) -> String {
    // Absent whenever the pointer is outside the window — normal, not an error.
    let Some(cursor) = window.cursor_position() else {
        return "cursor: off-window".to_string();
    };

    // Fails only on a degenerate viewport or projection.
    let Ok(world) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return "cursor: conversion failed".to_string();
    };

    format!(
        "screen {:.0}, {:.0}   →   world {:.0}, {:.0}",
        cursor.x, cursor.y, world.x, world.y
    )
}

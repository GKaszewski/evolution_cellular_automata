use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy_image::{Image, ImageSampler};

use crate::components::{Organism, Position, Predator, TileComponent};
use crate::plugins::simulation::SimulationSet;
use crate::resources::{AppState, Biome, FoodGrid, World, TILE_SIZE_IN_PIXELS};

pub struct RenderingPlugin;

#[derive(Resource)]
struct HeatmapHandle(Handle<Image>);

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_world, setup_heatmap).chain())
            .add_systems(
                Update,
                (
                    update_heatmap.after(SimulationSet),
                    handle_camera_movement,
                    handle_zoom,
                )
                    .run_if(in_state(AppState::Simulate)),
            );
    }
}

fn spawn_world(
    mut commands: Commands,
    world: Res<World>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tile_size = Vec2::new(TILE_SIZE_IN_PIXELS, TILE_SIZE_IN_PIXELS);

    let shape = meshes.add(Rectangle::new(tile_size.x, tile_size.y));

    for (i, tile) in world.grid.iter().enumerate() {
        let x = i % world.width;
        let y = i / world.width;

        let color = match tile.biome {
            Biome::Forest => Color::hsl(120.0, 1.0, 0.1),
            Biome::Desert => Color::hsl(60.0, 1.0, 0.5),
            Biome::Water => Color::hsl(240.0, 1.0, 0.5),
            Biome::Grassland => Color::hsl(100.0, 1.0, 0.7),
        };

        commands
            .spawn((Mesh2d(shape.clone()), MeshMaterial2d(materials.add(color))))
            .insert(TileComponent {
                biome: tile.biome.clone(),
            })
            .insert(Transform {
                translation: Vec3::new(x as f32 * tile_size.x, y as f32 * tile_size.y, 0.0),
                ..Default::default()
            });
    }

    let center_x = world.width as f32 * TILE_SIZE_IN_PIXELS / 2.0;
    let center_y = world.height as f32 * TILE_SIZE_IN_PIXELS / 2.0;

    commands.spawn((
        Camera2d::default(),
        Transform::from_xyz(center_x, center_y, 10.0),
    ));
}

fn setup_heatmap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    world: Res<World>,
) {
    let width = world.width as u32;
    let height = world.height as u32;

    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0u8; (width * height * 4) as usize],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image.texture_descriptor.usage |= TextureUsages::COPY_DST;

    let image_handle = images.add(image);

    let material = materials.add(ColorMaterial {
        texture: Some(image_handle.clone()),
        ..default()
    });

    let mesh = meshes.add(Rectangle::new(
        world.width as f32 * TILE_SIZE_IN_PIXELS,
        world.height as f32 * TILE_SIZE_IN_PIXELS,
    ));

    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_xyz(
            (world.width as f32 - 1.0) * TILE_SIZE_IN_PIXELS / 2.0,
            (world.height as f32 - 1.0) * TILE_SIZE_IN_PIXELS / 2.0,
            0.5,
        ),
    ));

    commands.insert_resource(HeatmapHandle(image_handle));
}

fn update_heatmap(
    heatmap: Res<HeatmapHandle>,
    mut images: ResMut<Assets<Image>>,
    organism_query: Query<&Position, (With<Organism>, Without<Predator>)>,
    predator_query: Query<&Position, With<Predator>>,
    food_grid: Res<FoodGrid>,
    world: Res<World>,
    mut org_counts: Local<Vec<u16>>,
    mut pred_counts: Local<Vec<u16>>,
) {
    let w = world.width;
    let h = world.height;
    let total = w * h;

    if org_counts.len() != total {
        *org_counts = vec![0u16; total];
        *pred_counts = vec![0u16; total];
    }

    for v in org_counts.iter_mut() {
        *v = 0;
    }
    for v in pred_counts.iter_mut() {
        *v = 0;
    }

    for pos in organism_query.iter() {
        let idx = pos.y * w + pos.x;
        org_counts[idx] = org_counts[idx].saturating_add(1);
    }
    for pos in predator_query.iter() {
        let idx = pos.y * w + pos.x;
        pred_counts[idx] = pred_counts[idx].saturating_add(1);
    }

    let Some(image) = images.get_mut(&heatmap.0) else {
        return;
    };
    let data = &mut image.data;

    for y in 0..h {
        for x in 0..w {
            let sim_idx = y * w + x;
            // Flip Y: sim y=0 is world bottom, but texture row 0 is screen top
            let tex_y = h - 1 - y;
            let tex_idx = (tex_y * w + x) * 4;

            let org = org_counts[sim_idx];
            let pred = pred_counts[sim_idx];
            let food = food_grid.0[sim_idx];

            if pred > 0 || org > 0 {
                // Entities present: fully opaque. Each entity contributes 50 brightness,
                // saturating at 5 organisms (green) or 5 predators (red).
                data[tex_idx] = (pred as u32 * 50).min(255) as u8; // R: predators
                data[tex_idx + 1] = (org as u32 * 50).min(255) as u8; // G: organisms
                data[tex_idx + 2] = 0;
                data[tex_idx + 3] = 255;
            } else if food > 0.5 {
                // Food only: subtle blue tint, semi-transparent so biome shows through.
                data[tex_idx] = 0;
                data[tex_idx + 1] = 0;
                data[tex_idx + 2] = ((food / 100.0) * 200.0).min(200.0) as u8;
                data[tex_idx + 3] = 80;
            } else {
                // Empty: fully transparent, biome tile visible.
                data[tex_idx] = 0;
                data[tex_idx + 1] = 0;
                data[tex_idx + 2] = 0;
                data[tex_idx + 3] = 0;
            }
        }
    }
}

fn handle_camera_movement(
    mut query: Query<(&mut Transform, &Camera)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for (mut transform, _) in query.iter_mut() {
        let mut translation = transform.translation;

        if keys.pressed(KeyCode::KeyW) {
            translation.y += 5.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            translation.y -= 5.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            translation.x -= 5.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            translation.x += 5.0;
        }

        transform.translation = translation;
    }
}

fn handle_zoom(
    mut query: Query<(&mut Transform, &mut Projection), With<Camera>>,
    mut scroll_events: EventReader<MouseWheel>,
    windows: Query<&Window>,
) {
    let scroll_delta: f32 = scroll_events
        .read()
        .map(|e| match e.unit {
            MouseScrollUnit::Line => e.y * 0.1,
            MouseScrollUnit::Pixel => e.y * 0.001,
        })
        .sum();

    if scroll_delta == 0.0 {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let window_size = Vec2::new(window.width(), window.height());
    // Flip Y: screen Y increases downward, world Y increases upward
    let cursor_screen_offset = Vec2::new(
        cursor_pos.x - window_size.x / 2.0,
        -(cursor_pos.y - window_size.y / 2.0),
    );

    // scroll up (delta > 0) → zoom in → scale decreases → factor < 1
    let zoom_factor = 1.0 - scroll_delta;

    for (mut transform, mut projection) in query.iter_mut() {
        if let Projection::Orthographic(ref mut ortho) = *projection {
            let old_scale = ortho.scale;
            let (new_pos, new_scale) = zoom_centered(
                transform.translation.truncate(),
                cursor_screen_offset,
                old_scale,
                zoom_factor,
                0.1,
                10.0,
            );
            ortho.scale = new_scale;
            transform.translation = new_pos.extend(transform.translation.z);
        }
    }
}

fn zoom_centered(
    camera_pos: Vec2,
    cursor_screen_offset: Vec2,
    old_scale: f32,
    zoom_factor: f32,
    min_scale: f32,
    max_scale: f32,
) -> (Vec2, f32) {
    let new_scale = (old_scale * zoom_factor).clamp(min_scale, max_scale);
    let delta = cursor_screen_offset * (old_scale - new_scale);
    (camera_pos + delta, new_scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_in_shifts_camera_toward_cursor() {
        // cursor 50px right of center, zoom in (factor < 1)
        let (pos, scale) = zoom_centered(
            Vec2::new(100.0, 100.0),
            Vec2::new(50.0, 0.0),
            1.0,
            0.9,
            0.1,
            10.0,
        );
        assert!((scale - 0.9).abs() < 1e-5);
        // delta = 50 * (1.0 - 0.9) = 5.0 — camera moves toward cursor
        assert!((pos.x - 105.0).abs() < 1e-4);
        assert!((pos.y - 100.0).abs() < 1e-4);
    }

    #[test]
    fn zoom_clamps_to_max() {
        let (_, scale) = zoom_centered(Vec2::ZERO, Vec2::ZERO, 9.5, 2.0, 0.1, 10.0);
        assert_eq!(scale, 10.0);
    }

    #[test]
    fn zoom_clamps_to_min() {
        let (_, scale) = zoom_centered(Vec2::ZERO, Vec2::ZERO, 0.15, 0.5, 0.1, 10.0);
        assert_eq!(scale, 0.1);
    }

    #[test]
    fn zoom_no_cursor_offset_does_not_translate() {
        let (pos, _) = zoom_centered(Vec2::new(50.0, 50.0), Vec2::ZERO, 1.0, 0.9, 0.1, 10.0);
        assert_eq!(pos, Vec2::new(50.0, 50.0));
    }
}

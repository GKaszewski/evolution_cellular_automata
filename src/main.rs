use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;

use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use rand::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Biome {
    Forest,
    Desert,
    Water,
    Grassland,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub biome: Biome,
    pub temperature: f32,
    pub humidity: f32,
    pub food_availabilty: f32,
}

impl Tile {
    pub fn regenerate_food(&mut self) {
        self.food_availabilty += (self.food_availabilty + 0.1).min(1.0);
    }
}

#[derive(Debug, Resource)]
pub struct World {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<Vec<Tile>>,
}

impl World {
    pub fn new(width: usize, height: usize) -> Self {
        let mut grid = vec![vec![]; height];
        for y in 0..height {
            for x in 0..width {
                let biome = match (x + y) % 4 {
                    0 => Biome::Forest,
                    1 => Biome::Desert,
                    // 2 => Biome::Water,
                    _ => Biome::Grassland,
                };
                grid[y].push(Tile {
                    biome,
                    temperature: 20.0,
                    humidity: 0.5,
                    food_availabilty: 1.0,
                });
            }
        }

        Self {
            width,
            height,
            grid,
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(10, 10)
    }
}

#[derive(Component)]
pub struct Organism {
    pub energy: f32,
    pub speed: f32,
    pub size: f32,
    pub reproduction_threshold: f32, // energy threshold for reproduction
    pub biome_tolerance: HashMap<Biome, f32>,
}

#[derive(Component)]
pub struct Predator {
    pub energy: f32,
    pub speed: f32,
    pub size: f32,
    pub hunting_efficiency: f32, // how much energy is consumed per kill
}

#[derive(Component)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Component)]
pub struct TileComponent {
    pub biome: Biome,
}

#[derive(Default, Resource)]
pub struct Generation(pub usize);

fn island_world() -> World {
    let mut world = World::new(10, 10);

    for y in 0..world.height {
        for x in 0..world.width {
            if x == 0 || x == world.width - 1 || y == 0 || y == world.height - 1 {
                world.grid[y][x].biome = Biome::Water;
            } else {
                if x % 2 == 0 && y % 2 == 0 {
                    world.grid[y][x].biome = Biome::Grassland;
                } else {
                    world.grid[y][x].biome = Biome::Forest;
                }
            }
        }
    }

    world
}

fn spawn_world(mut commands: Commands, world: Res<World>) {
    let tile_size = Vec2::new(32.0, 32.0);
    for (y, row) in world.grid.iter().enumerate() {
        for (x, tile) in row.iter().enumerate() {
            let color = match tile.biome {
                Biome::Forest => Color::linear_rgb(0.0, 255.0, 0.0),
                Biome::Desert => Color::linear_rgb(255.0, 255.0, 0.0),
                Biome::Water => Color::linear_rgb(0.0, 0.0, 255.0),
                Biome::Grassland => Color::linear_rgb(20.0, 255.0, 20.0),
            };

            commands
                .spawn(Sprite::from_color(color, tile_size))
                .insert(TileComponent {
                    biome: tile.biome.clone(),
                })
                .insert(Transform {
                    translation: Vec3::new(x as f32 * tile_size.x, y as f32 * tile_size.y, 0.0),
                    ..Default::default()
                });
        }
    }

    commands.spawn((Camera2d::default(), Transform::from_xyz(160.0, 160.0, 10.0)));
}

fn spawn_organisms(mut commands: Commands, world: Res<World>) {
    let mut rng = rand::thread_rng();
    let organism_count = 100;

    for _ in 0..organism_count {
        let x = rng.gen_range(0..world.width);
        let y = rng.gen_range(0..world.height);

        let mut biome_tolerance = HashMap::new();
        biome_tolerance.insert(Biome::Forest, rng.gen_range(0.8..1.2));
        biome_tolerance.insert(Biome::Desert, rng.gen_range(0.8..1.2));
        biome_tolerance.insert(Biome::Water, rng.gen_range(0.8..1.2));
        biome_tolerance.insert(Biome::Grassland, rng.gen_range(0.8..1.2));

        commands.spawn((
            Organism {
                energy: 3.0,
                speed: 1.0,
                size: rng.gen_range(0.5..1.5),
                reproduction_threshold: 5.0,
                biome_tolerance,
            },
            Position { x, y },
        ));
    }
}

fn spawn_predators(mut commands: Commands, world: Res<World>) {
    let mut rng = rand::thread_rng();
    let predator_count = 1;

    for _ in 0..predator_count {
        let x = rng.gen_range(0..world.width);
        let y = rng.gen_range(0..world.height);

        commands.spawn((
            Predator {
                energy: 15.0,
                speed: 1.5,
                size: 1.0,
                hunting_efficiency: 1.4,
            },
            Position { x, y },
        ));
    }
}

fn render_organisms(
    mut commands: Commands,
    query: Query<(Entity, &Position), (Added<Position>, Without<Predator>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tile_size = Vec2::new(32.0, 32.0);
    let organism_size = Vec2::new(16.0, 16.0);

    let shape = meshes.add(Circle::new((organism_size.x) / 2.0));

    let color = Color::linear_rgb(0.0, 155.0, 12.0);

    for (entity, position) in query.iter() {
        commands.entity(entity).insert((
            Mesh2d(shape.clone()),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(
                position.x as f32 * tile_size.x,
                position.y as f32 * tile_size.y,
                1.0,
            ),
        ));
    }
}

fn render_predators(
    mut commands: Commands,
    query: Query<(Entity, &Position), (Added<Position>, With<Predator>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tile_size = Vec2::new(32.0, 32.0);
    let organism_size = Vec2::new(16.0, 16.0);

    let color = Color::srgb(255.0, 0.0, 0.0);
    let shape = meshes.add(Rectangle::new(organism_size.x, organism_size.y));

    for (entity, position) in query.iter() {
        commands.entity(entity).insert((
            Mesh2d(shape.clone()),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(
                position.x as f32 * tile_size.x,
                position.y as f32 * tile_size.y,
                1.0,
            ),
        ));
    }
}

fn render_new_organisms(
    mut commands: Commands,
    query: Query<(Entity, &Position, &Organism), Added<Organism>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let tile_size = Vec2::new(32.0, 32.0);

    for (entity, position, organism) in query.iter() {
        let color = Color::linear_rgb(0.0, 127.0, 0.0);

        let shape = meshes.add(Circle::new((organism.size * 16.0) / 2.0));

        commands.entity(entity).insert((
            Mesh2d(shape),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(
                position.x as f32 * tile_size.x,
                position.y as f32 * tile_size.y,
                1.0,
            ),
        ));
    }
}

fn render_new_predators(
    mut commands: Commands,
    query: Query<(Entity, &Position, &Predator), Added<Predator>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let tile_size = Vec2::new(32.0, 32.0);

    for (entity, position, predator) in query.iter() {
        let color = Color::linear_rgb(127.0, 0.0, 0.0);

        let shape = meshes.add(Rectangle::new(predator.size * 16.0, predator.size * 16.0));

        commands.entity(entity).insert((
            Mesh2d(shape),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(
                position.x as f32 * tile_size.x,
                position.y as f32 * tile_size.y,
                1.0,
            ),
        ));
    }
}

fn organism_movement(mut query: Query<(&mut Position, &mut Organism)>, world: Res<World>) {
    let mut rng = rand::thread_rng();

    for (mut position, mut organism) in query.iter_mut() {
        if organism.energy <= 0.0 {
            continue;
        }

        let dx = rng.gen_range(-1..=1) as isize * organism.speed as isize;
        let dy = rng.gen_range(-1..=1) as isize * organism.speed as isize;

        let new_x = (position.x as isize + dx).clamp(0, (world.width - 1) as isize) as usize;
        let new_y = (position.y as isize + dy).clamp(0, (world.height - 1) as isize) as usize;

        position.x = new_x;
        position.y = new_y;

        organism.energy -= 0.1 * organism.speed * organism.size;
    }
}

fn predator_movement(
    mut predator_query: Query<(&mut Position, &mut Predator)>,
    prey_query: Query<(&Position, &Organism), Without<Predator>>,
    world: Res<World>,
) {
    let mut rng = rand::thread_rng();

    for (mut predator_position, mut predator) in predator_query.iter_mut() {
        if predator.energy <= 0.0 {
            continue; // Predator is dead
        }

        let mut closest_prey: Option<&Position> = None;
        let mut min_distance = f32::MAX;
        let predator_range_attack = 1.0;

        for (prey_position, _) in prey_query.iter() {
            let dx = predator_position.x as f32 - prey_position.x as f32;
            let dy = predator_position.y as f32 - prey_position.y as f32;
            let distance = dx * dx + dy * dy;

            if distance < min_distance && distance <= predator_range_attack {
                min_distance = distance;
                closest_prey = Some(prey_position);
            }
        }

        if let Some(prey_position) = closest_prey {
            let dx = prey_position.x as isize - predator_position.x as isize;
            let dy = prey_position.y as isize - predator_position.y as isize;

            predator_position.x = (predator_position.x as isize + dx.signum())
                .clamp(0, (world.width - 1) as isize) as usize;
            predator_position.y = (predator_position.y as isize + dy.signum())
                .clamp(0, (world.height - 1) as isize) as usize;
        } else {
            let dx = rng.gen_range(-1..=1);
            let dy = rng.gen_range(-1..=1);

            predator_position.x =
                (predator_position.x as isize + dx).clamp(0, (world.width - 1) as isize) as usize;
            predator_position.y =
                (predator_position.y as isize + dy).clamp(0, (world.height - 1) as isize) as usize;
        }

        predator.energy -= 0.1;
    }
}

fn despawn_dead_organisms(mut commands: Commands, query: Query<(Entity, &Organism)>) {
    for (entity, organism) in query.iter() {
        if organism.energy <= 0.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn despawn_dead_predators(mut commands: Commands, query: Query<(Entity, &Predator)>) {
    for (entity, predator) in query.iter() {
        if predator.energy <= 0.0 {
            println!("Predator died");
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn organism_sync(mut query: Query<(&Position, &mut Transform, &Organism)>) {
    for (position, mut transform, organism) in query.iter_mut() {
        transform.translation.x = position.x as f32 * 32.0;
        transform.translation.y = position.y as f32 * 32.0;
        transform.scale = Vec3::new(organism.size, organism.size, 1.0);
    }
}

fn predator_sync(mut query: Query<(&Position, &mut Transform, &Predator)>) {
    for (position, mut transform, predator) in query.iter_mut() {
        transform.translation.x = position.x as f32 * 32.0;
        transform.translation.y = position.y as f32 * 32.0;
        transform.scale = Vec3::new(predator.size, predator.size, 1.0);
    }
}

fn regenerate_food(mut world: ResMut<World>) {
    for row in world.grid.iter_mut() {
        for tile in row.iter_mut() {
            if tile.biome == Biome::Water {
                continue;
            }

            if tile.food_availabilty < 1500.0 {
                tile.regenerate_food();
            }
        }
    }
}

fn consume_food(mut world: ResMut<World>, mut query: Query<(Entity, &mut Organism, &Position)>) {
    let mut organisms_by_tile: HashMap<(usize, usize), Vec<(Entity, Mut<Organism>)>> =
        HashMap::new();

    for (entity, organism, position) in query.iter_mut() {
        organisms_by_tile
            .entry((position.x, position.y))
            .or_default()
            .push((entity, organism));
    }

    for ((x, y), organisms) in organisms_by_tile.iter_mut() {
        let tile = &mut world.grid[*y][*x];
        if tile.food_availabilty < 0.0 {
            continue;
        }

        // Largest organisms eat first (because JUNGLE RULES)
        organisms.sort_by(|a, b| {
            b.1.size
                .partial_cmp(&a.1.size)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut remaining_food = tile.food_availabilty;
        for (_, organism) in organisms.iter_mut() {
            if remaining_food <= 0.0 {
                break;
            }

            let food_needed = organism.size * 0.2 * organism.speed; // larger organisms need more food

            let food_consumed = food_needed.min(remaining_food);
            remaining_food -= food_consumed;
            organism.energy += food_consumed * 2.0; // Convert food to energy

            tile.food_availabilty -= food_consumed;
        }
    }
}

fn biome_adaptation(mut query: Query<(&mut Organism, &Position)>, world: Res<World>) {
    for (mut organism, position) in query.iter_mut() {
        let tile = &world.grid[position.y][position.x];
        let tolerance = organism.biome_tolerance.get(&tile.biome).unwrap_or(&1.0);

        match tile.biome {
            Biome::Forest => {
                organism.energy += 0.1 * tolerance; // forest are abundant in food
            }
            Biome::Desert => {
                organism.energy -= 0.1 / tolerance; // desert are scarce in food
            }
            Biome::Water => {
                organism.energy -= f32::MAX; // water is not a good place to be
            }
            Biome::Grassland => {
                organism.energy += 0.05 * tolerance; // grassland are good for grazing
            }
        }
    }
}

fn reproduction(mut commands: Commands, mut query: Query<(&mut Organism, &Position)>, world: Res<World>) {
    let mut rng = rand::thread_rng();

    for (mut organism, position) in query.iter_mut() {
        if organism.energy > organism.reproduction_threshold {
            let mutation_factor = 0.1;

            let mut biome_tolerance = organism.biome_tolerance.clone();
            for (_, tolerance) in biome_tolerance.iter_mut() {
                *tolerance *= 1.0 + rng.gen_range(-mutation_factor..mutation_factor);
            }

            let reproduction_threshold = organism.reproduction_threshold
                * (1.0 + rng.gen_range(-mutation_factor..mutation_factor));

            let child = Organism {
                energy: organism.energy / 2.0,
                speed: organism.speed * (1.1 + rng.gen_range(-mutation_factor..mutation_factor)),
                size: organism.size * (1.0 + rng.gen_range(-mutation_factor..mutation_factor)),
                reproduction_threshold,
                biome_tolerance,
            };

            let x_offset = rng.gen_range(-1..=1);
            let y_offset = rng.gen_range(-1..=1);

            let child_position = Position {
                x: (position.x as isize + x_offset).clamp(0, world.width as isize - 1) as usize,
                y: (position.y as isize + y_offset).clamp(0, world.height as isize - 1) as usize,
            };

            commands.spawn((child, child_position));

            organism.energy /= 2.0;
        }
    }
}

fn hunting(
    mut commands: Commands,
    mut predator_query: Query<(&mut Predator, &Position)>,
    prey_query: Query<(Entity, &Position, &Organism), Without<Predator>>,
) {
    for (mut predator, predator_position) in predator_query.iter_mut() {
        for (prey_entity, prey_position, prey) in prey_query.iter() {
            if predator_position.x == prey_position.x && predator_position.y == prey_position.y {
                let energy_gained = prey.size * predator.hunting_efficiency;
                predator.energy += energy_gained;

                //println!("Predator ate prey and gained {} energy", energy_gained);

                commands.entity(prey_entity).try_despawn_recursive();

                break;
            }
        }
    }
}

fn predator_reproduction(
    mut commands: Commands,
    mut query: Query<(&mut Predator, &Position)>,
    world: Res<World>,
) {
    let mut rng = rand::thread_rng();

    for (mut predator, position) in query.iter_mut() {
        if predator.energy > 120.0 {
            let mutation_factor = 0.05;

            let child = Predator {
                energy: predator.energy / 2.0,
                speed: predator.speed * (1.1 + rng.gen_range(-mutation_factor..mutation_factor)),
                size: predator.size * (1.0 + rng.gen_range(-mutation_factor..mutation_factor)),
                hunting_efficiency: predator.hunting_efficiency
                    * (1.0 + rng.gen_range(-mutation_factor..mutation_factor)),
            };

            let x_offset = rng.gen_range(-1..=1);
            let y_offset = rng.gen_range(-1..=1);

            let child_position = Position {
                x: (position.x as isize + x_offset).clamp(0, world.width as isize - 1) as usize,
                y: (position.y as isize + y_offset).clamp(0, world.height as isize - 1) as usize,
            };

            commands.spawn((child, child_position));

            predator.energy /= 2.0;
        }
    }
}

fn overcrowding(mut query: Query<(&mut Organism, &Position)>, mut predator_query: Query<(&mut Predator, &Position)>) {
    let overcrowding_threshold_for_organisms = 250;
    let overcrowding_threshold_for_predators = 10;
    
    let mut organisms_by_tile: HashMap<(usize, usize), Vec<Mut<Organism>>> = HashMap::new();

    for (organism, position) in query.iter_mut() {
        organisms_by_tile
            .entry((position.x, position.y))
            .or_default()
            .push(organism);
    }

    for (_, organisms) in organisms_by_tile.iter_mut() {
        if organisms.len() > overcrowding_threshold_for_organisms {
            organisms.sort_by(|a, b| {
                a.energy
                    .partial_cmp(&b.energy)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let num_to_remove = organisms.len() - overcrowding_threshold_for_organisms;
            for organism in organisms.iter_mut().take(num_to_remove) {
                organism.energy = -1.0;

                // println!("Organism died due to overcrowding");
            }
        }
    }

    let mut predators_by_tile: HashMap<(usize, usize), Vec<Mut<Predator>>> = HashMap::new();

    for (predator, position) in predator_query.iter_mut() {
        predators_by_tile
            .entry((position.x, position.y))
            .or_default()
            .push(predator);
    }

    for (_, predators) in predators_by_tile.iter_mut() {
        if predators.len() > overcrowding_threshold_for_predators {
            predators.sort_by(|a, b| {
                a.energy
                    .partial_cmp(&b.energy)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let num_to_remove = predators.len() - overcrowding_threshold_for_predators;
            for predator in predators.iter_mut().take(num_to_remove) {
                predator.energy = -1.0;

                // println!("Predator died due to overcrowding");
            }
        }
    }
}

#[allow(dead_code)]
fn print_energy(query: Query<(Entity, &Organism)>) {
    for (entity, organism) in query.iter() {
        println!("Energy: {} for entity {:?}", organism.energy, entity);
    }
}

#[allow(dead_code)]
fn print_how_many_organisms(query: Query<&Organism>) {
    println!("Number of organisms: {}", query.iter().count());
}

fn increment_generation(mut generation: ResMut<Generation>) {
    generation.0 += 1;
}

fn initialize_log_file() {
    let mut file = File::create("organism_data.csv").expect("Failed to create log file");
    writeln!(
        file,
        "generation,total_organisms,total_energy,avg_speed,avg_size,avg_reproduction_threshold,total_speed,total_size,total_reproduction_threshold,avg_energy"
    )
    .expect("Failed to write to log file");

    let mut predators_file = File::create("predator_data.csv").expect("Failed to create log file");
    writeln!(
        predators_file,
        "generation,total_predators,total_energy,avg_speed,avg_size,avg_reproduction_threshold,total_speed,total_size,total_reproduction_threshold,avg_energy"
    ).expect("Failed to write to log file");
}

fn log_organism_data(generation: Res<Generation>, query: Query<&Organism>, predator_query: Query<&Predator>) {
    let mut organisms_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("organism_data.csv")
        .expect("Failed to open log file");
    let mut predators_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("predator_data.csv")
        .expect("Failed to open log file");

    let mut total_organisms = 0;
    let mut total_energy = 0.0;
    let mut total_reproduction_threshold = 0.0;
    let mut total_speed = 0.0;
    let mut total_size = 0.0;

    for organism in query.iter() {
        total_organisms += 1;
        total_energy += organism.energy;
        total_reproduction_threshold += organism.reproduction_threshold;
        total_speed += organism.speed;
        total_size += organism.size;
    }

    if total_organisms > 0 {
        let avg_speed = total_speed / total_organisms as f32;
        let avg_size = total_size / total_organisms as f32;
        let avg_reproduction_threshold = total_reproduction_threshold / total_organisms as f32;

        writeln!(
            organisms_file,
            "{},{},{},{},{},{},{},{},{},{}",
            generation.0,
            total_organisms,
            total_energy,
            avg_speed,
            avg_size,
            avg_reproduction_threshold,
            total_speed,
            total_size,
            total_reproduction_threshold,
            total_energy / total_organisms as f32
        )
        .expect("Failed to write to log file");
    }

    let mut total_predators = 0;
    let mut total_predator_energy = 0.0;
    let mut total_predator_speed = 0.0;
    let mut total_predator_size = 0.0;

    for predator in predator_query.iter() {
        total_predators += 1;
        total_predator_energy += predator.energy;
        total_predator_speed += predator.speed;
        total_predator_size += predator.size;
    }

    if total_predators > 0 {
        let avg_predator_speed = total_predator_speed / total_predators as f32;
        let avg_predator_size = total_predator_size / total_predators as f32;

        writeln!(
            predators_file,
            "{},{},{},{},{},{},{},{},{},{}",
            generation.0,
            total_predators,
            total_predator_energy,
            avg_predator_speed,
            avg_predator_size,
            0.0,
            total_predator_speed,
            total_predator_size,
            0.0,
            total_predator_energy / total_predators as f32
        )
        .expect("Failed to write to log file");
    }
}

fn handle_camera_movement(
    mut query: Query<(&mut Transform, &Camera)>,
     keys: Res<ButtonInput<KeyCode>>,
) {
    for (mut transform, _) in query.iter_mut() {
        let mut translation = transform.translation;

        if keys.pressed(KeyCode::KeyW) {
            translation.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            translation.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            translation.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            translation.x += 1.0;
        }

        transform.translation = translation;
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(World::new(128, 128))
        .insert_resource(Generation(0))
        .add_systems(
            Startup,
            (
                spawn_world,
                spawn_organisms,
                spawn_predators,
                initialize_log_file,
            ),
        )
        .add_systems(Update, hunting)
        .add_systems(
            Update,
            (
                render_organisms,
                render_new_organisms,
                render_predators,
                organism_movement,
                predator_movement,
                organism_sync,
                predator_sync,
                despawn_dead_organisms,
                despawn_dead_predators,
                regenerate_food,
                consume_food,
                overcrowding,
                biome_adaptation,
                reproduction,
                predator_reproduction,
                increment_generation,
                log_organism_data,
                handle_camera_movement,
            )
                .after(hunting),
        )
        .run();
}

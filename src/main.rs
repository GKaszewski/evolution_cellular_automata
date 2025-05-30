#[allow(unused)]
use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::utils::hashbrown::HashMap;
use noise::NoiseFn;
use noise::Perlin;
use rand::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum AppState {
    #[default]
    Simulate,
    Finished,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
struct BiomeDataConfig {
    // multplier for food regeneration
    food_availabilty: f32,
    max_food_availabilty: f32,
}

#[derive(Deserialize, Debug, Resource, Serialize, Clone)]
pub struct Config {
    width: usize,
    height: usize,
    initial_organisms: usize,
    initial_predators: usize,
    headless: bool,
    log_data: bool,
    forest: BiomeDataConfig,
    desert: BiomeDataConfig,
    water: BiomeDataConfig,
    grassland: BiomeDataConfig,
    initial_organism_energy: f32,
    initial_predator_energy: f32,
    initial_organism_speed: f32,
    initial_predator_speed: f32,
    initial_organism_size: f32,
    initial_predator_size: f32,
    initial_organism_reproduction_threshold: f32,
    initial_predator_reproduction_threshold: f32,
    initial_predator_hunting_efficiency: f32,
    initial_predator_satiation_threshold: f32,
    organism_mutability: f32,
    predator_mutability: f32,
    overcrowding_threshold_for_organisms: usize,
    overcrowding_threshold_for_predators: usize,
    max_predator_energy: f32,
    predator_energy_decay_rate: f32,
    organism_reproduction_cooldown: f32,
    predator_reproduction_cooldown: f32,
    max_total_entities: usize, // max number of organisms and predators
    seed: u64,
    generation_limit: Option<usize>,
    printing: bool,
}

#[derive(Serialize)]
struct OrganismWithPosition {
    organism: Organism,
    position: Position,
}

#[derive(Serialize)]
struct PredatorWithPosition {
    predator: Predator,
    position: Position,
}

#[derive(Serialize)]
struct ExportData {
    config: Config,
    organisms: Vec<OrganismWithPosition>,
    predators: Vec<PredatorWithPosition>,
    world: World,
    generation: usize,
}

#[derive(Serialize)]
struct GenerationStats {
    generation: u32,
    organism_count: usize,
    predator_count: usize,
    organism_avg_size: f32,
    organism_avg_speed: f32,
    organism_avg_energy: f32,
    organism_avg_reproduction_threshold: f32,
    predator_avg_size: f32,
    predator_avg_speed: f32,
    predator_avg_energy: f32,
    predator_avg_reproduction_threshold: f32,
    predator_avg_hunting_efficiency: f32,
    predator_avg_satiation_threshold: f32,
    biome_tally: HashMap<Biome, f32>,
    average_food: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Copy)]
pub enum Biome {
    Forest,
    Desert,
    Water,
    Grassland,
}

impl Display for Biome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Biome::Forest => write!(f, "Forest"),
            Biome::Desert => write!(f, "Desert"),
            Biome::Water => write!(f, "Water"),
            Biome::Grassland => write!(f, "Grassland"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Tile {
    pub biome: Biome,
    pub temperature: f32,
    pub humidity: f32,
    pub food_availabilty: f32,
}

impl Tile {
    pub fn regenerate_food(&mut self, config: &Config) {
        match self.biome {
            Biome::Forest => {
                if self.food_availabilty > config.forest.max_food_availabilty {
                    return;
                }

                self.food_availabilty += config.forest.food_availabilty;
            }
            Biome::Desert => {
                if self.food_availabilty > config.desert.max_food_availabilty {
                    return;
                }

                self.food_availabilty += config.desert.food_availabilty;
            }
            Biome::Grassland => {
                if self.food_availabilty > config.grassland.max_food_availabilty {
                    return;
                }

                self.food_availabilty += config.grassland.food_availabilty;
            }
            _ => {}
        }
    }
}

#[derive(Debug, Resource, Serialize, Clone)]
pub struct World {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<Vec<Tile>>,
}

impl World {
    pub fn new(width: usize, height: usize, random_seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(random_seed);
        let seed = rng.gen::<u32>();

        let perlin = Perlin::new(seed);
        let scale = 10.0;

        let mut grid = vec![vec![]; height];
        for y in 0..height {
            for x in 0..width {
                let noise_value = perlin.get([x as f64 / scale, y as f64 / scale]);

                let biome = if noise_value < -0.3 {
                    Biome::Water
                } else if noise_value < -0.1 {
                    Biome::Desert
                } else if noise_value < 0.5 {
                    Biome::Grassland
                } else {
                    Biome::Forest
                };

                grid[y].push(Tile {
                    biome,
                    temperature: 20.0,
                    humidity: 0.5,
                    food_availabilty: rng.gen_range(1.0..100.0),
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
        Self::new(10, 10, 0)
    }
}

#[derive(Component, Serialize, Clone)]
pub struct Organism {
    pub energy: f32,
    pub speed: f32,
    pub size: f32,
    pub reproduction_threshold: f32, // energy threshold for reproduction
    pub reproduction_cooldown: f32,
    pub biome_tolerance: HashMap<Biome, f32>,
}

#[derive(Component, Serialize, Copy, Clone)]
pub struct Predator {
    pub energy: f32,
    pub speed: f32,
    pub size: f32,
    pub reproduction_threshold: f32, // energy threshold for reproduction
    pub hunting_efficiency: f32,     // how much energy is consumed per kill
    pub satiation_threshold: f32,    // only eat when energy is below this threshold
    pub reproduction_cooldown: f32,
}

#[derive(Component, Debug, Serialize, Copy, Clone)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Component)]
pub struct TileComponent {
    pub biome: Biome,
}

#[derive(Default, Resource, Serialize)]
pub struct Generation(pub usize);

const TILE_SIZE_IN_PIXELS: f32 = 32.0;

fn spawn_world(
    mut commands: Commands,
    world: Res<World>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tile_size = Vec2::new(TILE_SIZE_IN_PIXELS, TILE_SIZE_IN_PIXELS);

    let shape = meshes.add(Rectangle::new(tile_size.x, tile_size.y));

    for (y, row) in world.grid.iter().enumerate() {
        for (x, tile) in row.iter().enumerate() {
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
    }

    let center_x = world.width as f32 * TILE_SIZE_IN_PIXELS / 2.0;
    let center_y = world.height as f32 * TILE_SIZE_IN_PIXELS / 2.0;

    commands.spawn((
        Camera2d::default(),
        Transform::from_xyz(center_x, center_y, 10.0),
    ));
}

fn get_biome_tolerance(tile_biome: &Biome, seed: u64) -> HashMap<Biome, f32> {
    let mut biome_tolerance = HashMap::new();
    let mut rng = StdRng::seed_from_u64(seed);

    for biome in &[Biome::Forest, Biome::Desert, Biome::Water, Biome::Grassland] {
        let tolerance = if *biome == *tile_biome {
            rng.gen_range(1.0..1.5)
        } else {
            rng.gen_range(0.1..0.8)
        };

        biome_tolerance.insert(biome.clone(), tolerance);
    }

    biome_tolerance
}

fn spawn_organisms(mut commands: Commands, world: Res<World>, config: Res<Config>) {
    let mut rng = StdRng::seed_from_u64(config.seed);
    let organism_count = config.initial_organisms;

    for _ in 0..organism_count {
        let x = rng.gen_range(0..world.width);
        let y = rng.gen_range(0..world.height);

        let tile_biome = &world.grid[y][x].biome;

        let biome_tolerance = get_biome_tolerance(tile_biome, config.seed);

        commands.spawn((
            Organism {
                energy: config.initial_organism_energy,
                speed: config.initial_organism_speed,
                size: config.initial_organism_size,
                reproduction_threshold: config.initial_organism_reproduction_threshold,
                reproduction_cooldown: config.organism_reproduction_cooldown,
                biome_tolerance,
            },
            Position { x, y },
        ));
    }
}

fn spawn_predators(mut commands: Commands, world: Res<World>, config: Res<Config>) {
    let mut rng = StdRng::seed_from_u64(config.seed);
    let predator_count = config.initial_predators;

    for _ in 0..predator_count {
        let x = rng.gen_range(0..world.width);
        let y = rng.gen_range(0..world.height);

        commands.spawn((
            Predator {
                energy: config.initial_predator_energy,
                speed: config.initial_predator_speed,
                size: config.initial_predator_size,
                reproduction_threshold: config.initial_predator_reproduction_threshold,
                hunting_efficiency: config.initial_predator_hunting_efficiency,
                satiation_threshold: config.initial_predator_satiation_threshold,
                reproduction_cooldown: config.predator_reproduction_cooldown,
            },
            Position { x, y },
        ));
    }
}

fn render_organisms(
    mut commands: Commands,
    query: Query<(Entity, &Position), (Without<Predator>, Without<Mesh2d>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tile_size = Vec2::new(TILE_SIZE_IN_PIXELS, TILE_SIZE_IN_PIXELS);
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
    query: Query<(Entity, &Position), (Without<Organism>, Without<Mesh2d>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tile_size = Vec2::new(TILE_SIZE_IN_PIXELS, TILE_SIZE_IN_PIXELS);
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

fn organism_movement(
    mut query: Query<(&mut Position, &mut Organism)>,
    world: Res<World>,
    config: Res<Config>,
) {
    let directions: Vec<(isize, isize)> = vec![
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];

    let mut rng = StdRng::seed_from_u64(config.seed);

    for (mut position, mut organism) in query.iter_mut() {
        if organism.energy <= 0.0 {
            continue;
        }

        let mut best_direction = (0, 0);
        let mut best_cost = f32::MAX;

        for (dx, dy) in directions.iter() {
            let new_x = (position.x as isize + dx).clamp(0, (world.width - 1) as isize) as usize;
            let new_y = (position.y as isize + dy).clamp(0, (world.height - 1) as isize) as usize;
            let tile = &world.grid[new_y][new_x];

            let base_cost = match tile.biome {
                Biome::Water => 100.0,    // Very high cost; organisms avoid water
                Biome::Desert => 50.0,    // Moderately high cost
                Biome::Grassland => 10.0, // Low cost
                Biome::Forest => 20.0,    // Intermediate cost
            };

            let tolerance = organism.biome_tolerance.get(&tile.biome).unwrap_or(&1.0);
            let cost = base_cost / tolerance;

            let cost = cost + rng.gen_range(0.0..5.0);

            if cost < best_cost {
                best_cost = cost;
                best_direction = (*dx, *dy);
            }
        }

        position.x =
            (position.x as isize + best_direction.0).clamp(0, (world.width - 1) as isize) as usize;
        position.y =
            (position.y as isize + best_direction.1).clamp(0, (world.height - 1) as isize) as usize;

        let energy_to_consume = 0.1 * organism.speed * organism.size;

        organism.energy -= energy_to_consume;

        let tile = &world.grid[position.y][position.x];
        if tile.biome == Biome::Water {
            organism.energy = -1.0; // Organism dies in water
        }
    }
}

fn predator_movement(
    mut predator_query: Query<(&mut Position, &mut Predator)>,
    prey_query: Query<(&Position, &Organism), Without<Predator>>,
    world: Res<World>,
    config: Res<Config>,
) {
    let directions: Vec<(isize, isize)> = vec![
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    let mut rng = StdRng::seed_from_u64(config.seed);

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
            let mut best_direction = (0, 0);
            let mut best_cost = f32::MAX;

            for (dx, dy) in directions.iter() {
                let new_x = (predator_position.x as isize + dx).clamp(0, (world.width - 1) as isize)
                    as usize;
                let new_y = (predator_position.y as isize + dy)
                    .clamp(0, (world.height - 1) as isize) as usize;

                let tile = &world.grid[new_y][new_x];

                let cost = match tile.biome {
                    Biome::Water => 100.0,
                    Biome::Desert => 10.0,
                    Biome::Grassland => 5.0,
                    Biome::Forest => 6.0,
                };

                let cost = cost + rng.gen_range(0.0..5.0);

                if cost < best_cost {
                    best_cost = cost;
                    best_direction = (*dx, *dy);
                }
            }

            predator_position.x = (predator_position.x as isize + best_direction.0)
                .clamp(0, (world.width - 1) as isize) as usize;
            predator_position.y = (predator_position.y as isize + best_direction.1)
                .clamp(0, (world.height - 1) as isize) as usize;
        }

        predator.energy -= config.predator_energy_decay_rate * predator.speed * predator.size;
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
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn organism_sync(mut query: Query<(&Position, &mut Transform, &Organism)>) {
    for (position, mut transform, organism) in query.iter_mut() {
        transform.translation.x = position.x as f32 * TILE_SIZE_IN_PIXELS;
        transform.translation.y = position.y as f32 * TILE_SIZE_IN_PIXELS;
        transform.scale = Vec3::new(organism.size, organism.size, 1.0);
    }
}

fn predator_sync(mut query: Query<(&Position, &mut Transform, &Predator)>) {
    for (position, mut transform, predator) in query.iter_mut() {
        transform.translation.x = position.x as f32 * TILE_SIZE_IN_PIXELS;
        transform.translation.y = position.y as f32 * TILE_SIZE_IN_PIXELS;
        transform.scale = Vec3::new(predator.size, predator.size, 1.0);
    }
}

fn regenerate_food(mut world: ResMut<World>, config: Res<Config>) {
    for row in world.grid.iter_mut() {
        for tile in row.iter_mut() {
            tile.regenerate_food(&config);
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

fn reproduction(
    mut commands: Commands,
    mut query: Query<(&mut Organism, &Position)>,
    predators_query: Query<&Predator>,
    world: Res<World>,
    config: Res<Config>,
) {
    let organisms_count = query.iter().count();
    let predators_count = predators_query.iter().count();
    let total_entities = organisms_count + predators_count;

    if total_entities >= config.max_total_entities {
        if config.printing {
            println!("Max entities reached, not spawning organism");
        }
        return;
    }

    let mut rng = StdRng::seed_from_u64(config.seed);

    for (mut organism, position) in query.iter_mut() {
        if organism.reproduction_cooldown > 0.0 {
            organism.reproduction_cooldown -= 1.0;
            continue;
        }

        if organism.energy > organism.reproduction_threshold {
            let mutation_factor = config.organism_mutability;

            let tile_biome = &world.grid[position.y][position.x].biome;

            let mut biome_tolerance = get_biome_tolerance(tile_biome, config.seed);
            for (_, tolerance) in biome_tolerance.iter_mut() {
                *tolerance *= 1.0 + rng.gen_range(-mutation_factor..mutation_factor);
            }

            let reproduction_threshold = organism.reproduction_threshold
                * (1.0 + rng.gen_range(-mutation_factor..mutation_factor));

            let muated_size =
                organism.size * (1.0 + rng.gen_range(-mutation_factor..mutation_factor));
            let size = muated_size.max(0.1); // to avoid negative size
            let mutated_speed =
                organism.speed * (1.1 + rng.gen_range(-mutation_factor..mutation_factor));
            let penalty = size * 0.1;
            let speed = (mutated_speed - penalty).max(0.1); // to avoid negative speed

            let mutated_cooldown = (config.organism_reproduction_cooldown
                * (1.0 + rng.gen_range(-mutation_factor..mutation_factor)))
            .max(1.0); // min 1 tick

            let child = Organism {
                energy: organism.energy / 2.0,
                speed: speed,
                size: size,
                reproduction_threshold,
                biome_tolerance,
                reproduction_cooldown: mutated_cooldown,
            };

            let x_offset = rng.gen_range(-1..=1);
            let y_offset = rng.gen_range(-1..=1);

            let child_position = Position {
                x: (position.x as isize + x_offset).clamp(0, world.width as isize - 1) as usize,
                y: (position.y as isize + y_offset).clamp(0, world.height as isize - 1) as usize,
            };

            commands.spawn((child, child_position));

            organism.energy /= 2.0;
            organism.reproduction_cooldown = config.organism_reproduction_cooldown;
        }
    }
}

fn hunting(
    mut commands: Commands,
    mut predator_query: Query<(&mut Predator, &Position)>,
    prey_query: Query<(Entity, &Position, &Organism), Without<Predator>>,
    config: Res<Config>,
) {
    for (mut predator, predator_position) in predator_query.iter_mut() {
        if predator.energy >= predator.satiation_threshold {
            continue;
        }

        for (prey_entity, prey_position, prey) in prey_query.iter() {
            if predator_position.x == prey_position.x && predator_position.y == prey_position.y {
                let energy_gained = prey.size * predator.hunting_efficiency;
                predator.energy = (predator.energy + energy_gained).min(config.max_predator_energy);

                commands.entity(prey_entity).try_despawn_recursive();

                break;
            }
        }
    }
}

fn predator_reproduction(
    mut commands: Commands,
    mut query: Query<(&mut Predator, &Position)>,
    organisms_query: Query<&Organism>,
    world: Res<World>,
    config: Res<Config>,
) {
    let predators_count = query.iter().count();
    let organisms_count = organisms_query.iter().count();
    let total_entities = predators_count + organisms_count;

    if total_entities >= config.max_total_entities {
        if config.printing {
            println!("Max entities reached, not spawning predator");
        }
        return;
    }

    let mut rng = StdRng::seed_from_u64(config.seed);

    for (mut predator, position) in query.iter_mut() {
        if predator.reproduction_cooldown > 0.0 {
            predator.reproduction_cooldown -= 1.0;
            continue;
        }

        if predator.energy > predator.reproduction_threshold {
            let mutation_factor = config.predator_mutability;

            let muated_size =
                predator.size * (1.0 + rng.gen_range(-mutation_factor..mutation_factor));
            let size = muated_size.max(0.1); // to avoid negative size

            let mutated_speed =
                predator.speed * (1.1 + rng.gen_range(-mutation_factor..mutation_factor));
            let penalty = size * 0.1;
            let speed = (mutated_speed - penalty).max(0.1); // to avoid negative speed

            let reproduction_cooldown = (config.predator_reproduction_cooldown
                * (1.0 + rng.gen_range(-mutation_factor..mutation_factor)))
            .max(1.0); // min 1 tick

            let child = Predator {
                energy: predator.energy / 2.0,
                speed: speed,
                size: size,
                hunting_efficiency: predator.hunting_efficiency
                    * (1.0 + rng.gen_range(-mutation_factor..mutation_factor)),
                satiation_threshold: predator.satiation_threshold
                    * (1.0 + rng.gen_range(-mutation_factor..mutation_factor)),
                reproduction_threshold: predator.reproduction_threshold
                    * (1.0 + rng.gen_range(-mutation_factor..mutation_factor)),
                reproduction_cooldown,
            };

            let x_offset = rng.gen_range(-1..=1);
            let y_offset = rng.gen_range(-1..=1);

            let child_position = Position {
                x: (position.x as isize + x_offset).clamp(0, world.width as isize - 1) as usize,
                y: (position.y as isize + y_offset).clamp(0, world.height as isize - 1) as usize,
            };

            commands.spawn((child, child_position));

            predator.energy /= 2.0;
            predator.reproduction_cooldown = config.predator_reproduction_cooldown;
        }
    }
}

fn overcrowding(
    mut query: Query<(&mut Organism, &Position)>,
    mut predator_query: Query<(&mut Predator, &Position)>,
    config: Res<Config>,
) {
    let overcrowding_threshold_for_organisms = config.overcrowding_threshold_for_organisms;
    let overcrowding_threshold_for_predators = config.overcrowding_threshold_for_predators;

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

                if config.printing {
                    println!("Organism died due to overcrowding");
                }
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

                if config.printing {
                    println!("Predator died due to overcrowding");
                }
            }
        }
    }
}

fn increment_generation(mut generation: ResMut<Generation>) {
    generation.0 += 1;
}

fn initialize_log_file(config: Res<Config>) {
    if !config.log_data {
        return;
    }

    let world_file = File::create("world_data.jsonl").expect("Failed to create log file");
    world_file.set_len(0).expect("Failed to clear log file");
}

fn log_world_data(
    config: Res<Config>,
    world: Res<World>,
    generation: Res<Generation>,
    organisms_query: Query<(&Organism, &Position)>,
    predators_query: Query<(&Predator, &Position)>,
) {
    if !config.log_data {
        return;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("world_data.jsonl")
        .expect("Failed to open log file");

    let organisms_with_position = organisms_query
        .iter()
        .map(|(organism, position)| OrganismWithPosition {
            organism: organism.clone(),
            position: position.clone(),
        })
        .collect::<Vec<_>>();

    let predators_with_position = predators_query
        .iter()
        .map(|(predator, position)| PredatorWithPosition {
            predator: predator.clone(),
            position: position.clone(),
        })
        .collect::<Vec<_>>();

    let export_data = ExportData {
        generation: generation.0,
        world: world.clone(),
        config: config.clone(),
        organisms: organisms_with_position,
        predators: predators_with_position,
    };

    let json = serde_json::to_string(&export_data).expect("Failed to serialize data");

    writeln!(file, "{}", json).expect("Failed to write to log file");
}

fn log_preprocessed_world_data(
    config: Res<Config>,
    world: Res<World>,
    generation: Res<Generation>,
    organisms_query: Query<(&Organism, &Position)>,
    predators_query: Query<(&Predator, &Position)>,
) {
    if !config.log_data {
        return;
    }

    let mut biome_tally = HashMap::new();
    let mut organism_count = 0;
    let mut predator_count = 0;

    let mut organism_size_sum = 0.0;
    let mut organism_speed_sum = 0.0;
    let mut organism_energy_sum = 0.0;
    let mut organism_repro_sum = 0.0;

    for (organism, _) in organisms_query.iter() {
        organism_count += 1;
        organism_size_sum += organism.size;
        organism_speed_sum += organism.speed;
        organism_energy_sum += organism.energy;
        organism_repro_sum += organism.reproduction_threshold;

        for (biome, tolerance) in &organism.biome_tolerance {
            *biome_tally.entry(biome.clone()).or_insert(0.0) += tolerance;
        }
    }

    let mut predator_size_sum = 0.0;
    let mut predator_speed_sum = 0.0;
    let mut predator_energy_sum = 0.0;
    let mut predator_repro_sum = 0.0;
    let mut predator_hunting_sum = 0.0;
    let mut predator_satiation_sum = 0.0;

    for (predator, _) in predators_query.iter() {
        predator_count += 1;
        predator_size_sum += predator.size;
        predator_speed_sum += predator.speed;
        predator_energy_sum += predator.energy;
        predator_repro_sum += predator.reproduction_threshold;
        predator_hunting_sum += predator.hunting_efficiency;
        predator_satiation_sum += predator.satiation_threshold;
    }

    let total_tiles = (config.width * config.height) as f32;
    let total_food: f32 = world
        .grid
        .iter()
        .flat_map(|row| row.iter())
        .map(|tile| tile.food_availabilty)
        .sum();

    let summary = GenerationStats {
        generation: generation.0 as u32,
        organism_count,
        predator_count,
        organism_avg_size: organism_size_sum / organism_count.max(1) as f32,
        organism_avg_speed: organism_speed_sum / organism_count.max(1) as f32,
        organism_avg_energy: organism_energy_sum / organism_count.max(1) as f32,
        organism_avg_reproduction_threshold: organism_repro_sum / organism_count.max(1) as f32,
        predator_avg_size: predator_size_sum / predator_count.max(1) as f32,
        predator_avg_speed: predator_speed_sum / predator_count.max(1) as f32,
        predator_avg_energy: predator_energy_sum / predator_count.max(1) as f32,
        predator_avg_reproduction_threshold: predator_repro_sum / predator_count.max(1) as f32,
        predator_avg_hunting_efficiency: predator_hunting_sum / predator_count.max(1) as f32,
        predator_avg_satiation_threshold: predator_satiation_sum / predator_count.max(1) as f32,
        biome_tally,
        average_food: total_food / total_tiles,
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("summary_data.jsonl")
        .expect("Failed to open summary log file");

    writeln!(
        file,
        "{}",
        serde_json::to_string(&summary).expect("Failed to serialize summary data")
    )
    .expect("Failed to write summary data to log file");
}

#[allow(unused)]
fn run_if_any_organisms(query: Query<(&Organism, &Predator)>) -> bool {
    query.iter().count() > 0
}

fn run_for_x_generations(
    generation: Res<Generation>,
    config: Res<Config>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let generation_limit = config.generation_limit;

    if let Some(limit) = generation_limit {
        if config.printing {
            println!("Generation: {}", generation.0);
        }

        if generation.0 >= limit {
            next_state.set(AppState::Finished);
        }
    }
}

fn kill_over_limit_organisms(
    mut commands: Commands,
    organisms_query: Query<(Entity, &Organism)>,
    predators_query: Query<(Entity, &Predator)>,
    config: Res<Config>,
) {
    let limit = config.max_total_entities;
    let total_entities = organisms_query.iter().count() + predators_query.iter().count();
    let over_limit = total_entities as i32 - limit as i32;
    if over_limit > 0 {
        let mut rng = StdRng::seed_from_u64(config.seed);
        let mut entities_to_kill = Vec::new();

        for (entity, _) in organisms_query.iter() {
            entities_to_kill.push(entity);
        }

        for (entity, _) in predators_query.iter() {
            entities_to_kill.push(entity);
        }

        entities_to_kill.shuffle(&mut rng);

        for entity in entities_to_kill.iter().take(over_limit as usize) {
            commands.entity(*entity).despawn_recursive();
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

fn load_config() -> Result<Config, Box<dyn Error>> {
    let exe_dir = std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Executable must be in a directory")
        .to_path_buf();

    let config_path = exe_dir.join("config.toml");

    let config = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config)?;

    Ok(config)
}

#[allow(dead_code, unused)]
fn default_config() -> Config {
    Config {
        width: 10,
        height: 10,
        initial_organisms: 10,
        initial_predators: 1,
        headless: false,
        log_data: false,
        forest: BiomeDataConfig {
            food_availabilty: 1.0,
            max_food_availabilty: 100.0,
        },
        desert: BiomeDataConfig {
            food_availabilty: 1.0,
            max_food_availabilty: 100.0,
        },
        water: BiomeDataConfig {
            food_availabilty: 1.0,
            max_food_availabilty: 100.0,
        },
        grassland: BiomeDataConfig {
            food_availabilty: 1.0,
            max_food_availabilty: 100.0,
        },
        initial_organism_energy: 100.0,
        initial_predator_energy: 100.0,
        initial_organism_speed: 1.0,
        initial_predator_speed: 1.0,
        initial_organism_size: 1.0,
        initial_predator_size: 1.0,
        initial_organism_reproduction_threshold: 100.0,
        initial_predator_reproduction_threshold: 100.0,
        initial_predator_hunting_efficiency: 1.0,
        initial_predator_satiation_threshold: 100.0,
        organism_mutability: 0.1,
        predator_mutability: 0.1,
        overcrowding_threshold_for_organisms: 10,
        overcrowding_threshold_for_predators: 10,
        predator_energy_decay_rate: 0.5,
        max_predator_energy: 1500.0,
        organism_reproduction_cooldown: 0.5,
        predator_reproduction_cooldown: 0.5,
        seed: 0,
        max_total_entities: 1000,
        generation_limit: None,
        printing: false,
    }
}

fn get_config() -> Config {
    #[cfg(target_arch = "wasm32")]
    let config = default_config();
    #[cfg(not(target_arch = "wasm32"))]
    let config = load_config().expect("Failed to load config file");

    config
}

fn exit_app(mut exit: EventWriter<AppExit>, app_state: Res<State<AppState>>) {
    if app_state.get() == &AppState::Finished {
        exit.send(AppExit::Success);
    }
}

fn print_simulation_progress(
    generation: Res<Generation>,
    config: Res<Config>,
    app_state: Res<State<AppState>>,
    organisms_query: Query<&Organism>,
    predators_query: Query<&Predator>,
) {
    if app_state.get() == &AppState::Simulate {
        let organisms_count = organisms_query.iter().count();
        let predators_count = predators_query.iter().count();
        let total_entities = organisms_count + predators_count;
        println!(
            "Generation: {} / {}, Total entities: {}, Organisms: {}, Predators: {}",
            generation.0,
            config.generation_limit.unwrap_or(0),
            total_entities,
            organisms_count,
            predators_count
        );
    }
}

fn main() {
    let config = get_config();

    println!("{:?}", config);

    let headless = config.headless;
    let mut app = App::new();

    match headless {
        true => {
            app.add_plugins((MinimalPlugins, StatesPlugin));
        }
        false => {
            app.add_plugins(DefaultPlugins);
        }
    }

    app.insert_resource(World::new(config.width, config.height, config.seed))
        .insert_resource(config)
        .insert_resource(Generation(0))
        .init_state::<AppState>()
        .add_systems(
            Startup,
            (
                spawn_world,
                spawn_organisms,
                spawn_predators,
                initialize_log_file,
            ),
        )
        .add_systems(Update, (hunting).run_if(in_state(AppState::Simulate)))
        .add_systems(
            Update,
            (
                render_organisms,
                render_predators,
                organism_movement,
                predator_movement,
                despawn_dead_organisms,
                despawn_dead_predators,
                organism_sync,
                predator_sync,
                regenerate_food,
                consume_food,
                overcrowding,
                biome_adaptation,
                reproduction,
                predator_reproduction,
                increment_generation,
                log_world_data,
                handle_camera_movement,
            )
                .after(hunting)
                .run_if(in_state(AppState::Simulate)),
        )
        // .add_systems(
        //     Update,
        //     kill_over_limit_organisms
        //         .after(reproduction)
        //         .after(predator_reproduction)
        //         .after(overcrowding),
        // )
        // .add_systems(
        //     Update,
        //     log_preprocessed_world_data
        //         .after(despawn_dead_organisms)
        //         .after(despawn_dead_predators)
        //         .run_if(in_state(AppState::Simulate)),
        // )
        .add_systems(Update, run_for_x_generations.after(increment_generation))
        .add_systems(
            Update,
            print_simulation_progress
                .run_if(in_state(AppState::Simulate))
                .after(kill_over_limit_organisms),
        )
        .add_systems(Update, exit_app.run_if(in_state(AppState::Finished)))
        .run();
}

// ro3noleglosc systemow

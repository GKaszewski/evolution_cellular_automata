use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashSet;
use rand::prelude::*;
use rand::rngs::SmallRng;

use crate::components::{EntityRng, Organism, Position, Predator};
use crate::resources::{
    AppState, Biome, Config, FoodGrid, Generation, PopulationCount, PredatorSpatialIndex,
    ReproductionRng, SpatialIndex, SpawnRng, World,
};
use crate::utils::{get_biome_tolerance, DIRECTIONS};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationSet;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_organisms, spawn_predators))
            .add_systems(
                Update,
                (
                    update_population_count,
                    (organism_movement, predator_movement),
                    (rebuild_spatial_index, rebuild_predator_spatial_index),
                    hunting,
                    (consume_food, biome_adaptation, regenerate_food),
                    (despawn_dead_organisms, despawn_dead_predators),
                    (organism_overcrowding, predator_overcrowding),
                    (reproduction, predator_reproduction),
                    kill_over_limit_organisms,
                    increment_generation,
                    run_for_x_generations,
                    print_simulation_progress,
                )
                    .chain()
                    .run_if(in_state(AppState::Simulate))
                    .in_set(SimulationSet),
            )
            .add_systems(Update, exit_app.run_if(in_state(AppState::Finished)));
    }
}

fn spawn_organisms(
    mut commands: Commands,
    world: Res<World>,
    config: Res<Config>,
    mut rng: ResMut<SpawnRng>,
) {
    let organism_count = config.organism.initial_organisms;

    for _ in 0..organism_count {
        let x = rng.0.gen_range(0..world.width);
        let y = rng.0.gen_range(0..world.height);

        let tile_biome = world.tile(x, y).biome;

        let biome_tolerance = get_biome_tolerance(tile_biome, &mut rng.0);
        let entity_seed: u64 = rng.0.gen();

        commands.spawn((
            Organism {
                energy: config.organism.initial_organism_energy,
                speed: config.organism.initial_organism_speed,
                size: config.organism.initial_organism_size,
                reproduction_threshold: config.organism.initial_organism_reproduction_threshold,
                reproduction_cooldown: config.organism.organism_reproduction_cooldown,
                biome_tolerance,
            },
            Position { x, y },
            EntityRng(SmallRng::seed_from_u64(entity_seed)),
        ));
    }
}

fn spawn_predators(
    mut commands: Commands,
    world: Res<World>,
    config: Res<Config>,
    mut rng: ResMut<SpawnRng>,
) {
    let predator_count = config.predator.initial_predators;

    for _ in 0..predator_count {
        let x = rng.0.gen_range(0..world.width);
        let y = rng.0.gen_range(0..world.height);

        let entity_seed: u64 = rng.0.gen();

        commands.spawn((
            Predator {
                energy: config.predator.initial_predator_energy,
                speed: config.predator.initial_predator_speed,
                size: config.predator.initial_predator_size,
                reproduction_threshold: config.predator.initial_predator_reproduction_threshold,
                hunting_efficiency: config.predator.initial_predator_hunting_efficiency,
                satiation_threshold: config.predator.initial_predator_satiation_threshold,
                reproduction_cooldown: config.predator.predator_reproduction_cooldown,
            },
            Position { x, y },
            EntityRng(SmallRng::seed_from_u64(entity_seed)),
        ));
    }
}

fn organism_movement(
    mut query: Query<(&mut Position, &mut Organism, &mut EntityRng)>,
    world: Res<World>,
) {
    query
        .par_iter_mut()
        .for_each(|(mut position, mut organism, mut entity_rng)| {
            if organism.energy <= 0.0 {
                return;
            }

            let rng = &mut entity_rng.0;
            let base_moves = organism.speed.floor() as u32;
            let extra = u32::from(rng.gen::<f32>() < organism.speed.fract());
            let total_moves = (base_moves + extra).max(1);

            for _ in 0..total_moves {
                let mut best_direction = (0isize, 0isize);
                let mut best_cost = f32::MAX;

                for &(dx, dy) in DIRECTIONS.iter() {
                    let new_x =
                        (position.x as isize + dx).clamp(0, (world.width - 1) as isize) as usize;
                    let new_y =
                        (position.y as isize + dy).clamp(0, (world.height - 1) as isize) as usize;
                    let tile = world.tile(new_x, new_y);

                    let base_cost = match tile.biome {
                        Biome::Water => 100.0,
                        Biome::Desert => 50.0,
                        Biome::Grassland => 10.0,
                        Biome::Forest => 20.0,
                    };

                    let tolerance = organism.biome_tolerance[tile.biome.idx()];
                    let cost = base_cost / tolerance + rng.gen_range(0.0..5.0_f32);

                    if cost < best_cost {
                        best_cost = cost;
                        best_direction = (dx, dy);
                    }
                }

                position.x = (position.x as isize + best_direction.0)
                    .clamp(0, (world.width - 1) as isize) as usize;
                position.y = (position.y as isize + best_direction.1)
                    .clamp(0, (world.height - 1) as isize) as usize;

                organism.energy -= 0.1 * organism.speed * organism.size;

                let tile = world.tile(position.x, position.y);
                if tile.biome == Biome::Water {
                    organism.energy = -1.0;
                    break;
                }
            }
        });
}

fn predator_movement(
    mut predator_query: Query<(&mut Position, &mut Predator, &mut EntityRng)>,
    world: Res<World>,
    config: Res<Config>,
    index: Res<SpatialIndex>,
) {
    let radius = config.predator.predator_seek_radius as isize;

    predator_query.par_iter_mut().for_each(
        |(mut predator_position, mut predator, mut entity_rng)| {
            if predator.energy <= 0.0 {
                return;
            }

            let rng = &mut entity_rng.0;
            let base_moves = predator.speed.floor() as u32;
            let extra = u32::from(rng.gen::<f32>() < predator.speed.fract());
            let total_moves = (base_moves + extra).max(1);

            for _ in 0..total_moves {
                let mut closest_offset: Option<(isize, isize)> = None;
                let mut min_dist_sq = i32::MAX;

                for ddx in -radius..=radius {
                    for ddy in -radius..=radius {
                        let nx = (predator_position.x as isize + ddx)
                            .clamp(0, world.width as isize - 1)
                            as usize;
                        let ny = (predator_position.y as isize + ddy)
                            .clamp(0, world.height as isize - 1)
                            as usize;
                        if !index.get(nx, ny).is_empty() {
                            let d = (ddx * ddx + ddy * ddy) as i32;
                            if d < min_dist_sq {
                                min_dist_sq = d;
                                closest_offset = Some((ddx, ddy));
                            }
                        }
                    }
                }

                if let Some((dx, dy)) = closest_offset {
                    predator_position.x = (predator_position.x as isize + dx.signum())
                        .clamp(0, world.width as isize - 1)
                        as usize;
                    predator_position.y = (predator_position.y as isize + dy.signum())
                        .clamp(0, world.height as isize - 1)
                        as usize;
                } else {
                    let mut best_direction = (0isize, 0isize);
                    let mut best_cost = f32::MAX;

                    for &(dx, dy) in DIRECTIONS.iter() {
                        let new_x = (predator_position.x as isize + dx)
                            .clamp(0, world.width as isize - 1)
                            as usize;
                        let new_y = (predator_position.y as isize + dy)
                            .clamp(0, world.height as isize - 1)
                            as usize;
                        let tile = world.tile(new_x, new_y);
                        let cost = match tile.biome {
                            Biome::Water => 100.0,
                            Biome::Desert => 10.0,
                            Biome::Grassland => 5.0,
                            Biome::Forest => 6.0,
                        } + rng.gen_range(0.0..5.0_f32);
                        if cost < best_cost {
                            best_cost = cost;
                            best_direction = (dx, dy);
                        }
                    }

                    predator_position.x = (predator_position.x as isize + best_direction.0)
                        .clamp(0, world.width as isize - 1)
                        as usize;
                    predator_position.y = (predator_position.y as isize + best_direction.1)
                        .clamp(0, world.height as isize - 1)
                        as usize;
                }

                predator.energy -=
                    config.predator.predator_energy_decay_rate * predator.speed * predator.size;
            }
        },
    );
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

fn regenerate_food(world: Res<World>, mut food_grid: ResMut<FoodGrid>, config: Res<Config>) {
    for (tile, food) in world.grid.iter().zip(food_grid.0.iter_mut()) {
        match tile.biome {
            Biome::Forest => {
                if *food <= config.world.forest.max_food_availability {
                    *food += config.world.forest.food_availability;
                }
            }
            Biome::Desert => {
                if *food <= config.world.desert.max_food_availability {
                    *food += config.world.desert.food_availability;
                }
            }
            Biome::Grassland => {
                if *food <= config.world.grassland.max_food_availability {
                    *food += config.world.grassland.food_availability;
                }
            }
            _ => {}
        }
    }
}

fn consume_food(
    mut food_grid: ResMut<FoodGrid>,
    index: Res<SpatialIndex>,
    mut query: Query<&mut Organism>,
    mut scratch: Local<Vec<(Entity, f32)>>,
) {
    for (i, cell) in index.cells.iter().enumerate() {
        if cell.is_empty() {
            continue;
        }
        let food = food_grid.0[i];
        if food <= 0.0 {
            continue;
        }

        scratch.clear();
        for &entity in cell.iter() {
            if let Ok(organism) = query.get(entity) {
                scratch.push((entity, organism.size));
            }
        }
        if scratch.is_empty() {
            continue;
        }

        scratch.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut remaining = food;
        for &(entity, _) in scratch.iter() {
            if remaining <= 0.0 {
                break;
            }
            if let Ok(mut organism) = query.get_mut(entity) {
                let needed = organism.size * 0.2 * organism.speed;
                let consumed = needed.min(remaining);
                remaining -= consumed;
                organism.energy += consumed * 2.0;
            }
        }
        food_grid.0[i] = remaining;
    }
}

fn biome_adaptation(mut query: Query<(&mut Organism, &Position)>, world: Res<World>) {
    query.par_iter_mut().for_each(|(mut organism, position)| {
        let tile = world.tile(position.x, position.y);
        let tolerance = organism.biome_tolerance[tile.biome.idx()];

        match tile.biome {
            Biome::Forest => {
                organism.energy += 0.1 * tolerance;
            }
            Biome::Desert => {
                organism.energy -= 0.1 / tolerance;
            }
            Biome::Water => {
                organism.energy = -1.0;
            }
            Biome::Grassland => {
                organism.energy += 0.05 * tolerance;
            }
        }
    });
}

fn update_population_count(
    organisms_query: Query<&Organism>,
    predators_query: Query<&Predator>,
    mut pop: ResMut<PopulationCount>,
) {
    pop.organisms = organisms_query.iter().count();
    pop.predators = predators_query.iter().count();
}

fn reproduction(
    mut commands: Commands,
    mut query: Query<(&mut Organism, &Position)>,
    world: Res<World>,
    config: Res<Config>,
    mut rng: ResMut<ReproductionRng>,
    pop: Res<PopulationCount>,
) {
    let total_entities = pop.organisms + pop.predators;

    if total_entities >= config.world.max_total_entities {
        if config.world.printing {
            println!("Max entities reached, not spawning organism");
        }
        return;
    }

    for (mut organism, position) in query.iter_mut() {
        if organism.energy <= 0.0 {
            continue;
        }
        if organism.reproduction_cooldown > 0.0 {
            organism.reproduction_cooldown -= 1.0;
            continue;
        }

        if organism.energy > organism.reproduction_threshold {
            let mutation_factor = config.organism.organism_mutability;

            let mut biome_tolerance = organism.biome_tolerance;
            for tolerance in biome_tolerance.iter_mut() {
                *tolerance *= 1.0 + rng.0.gen_range(-mutation_factor..mutation_factor);
                *tolerance = tolerance.max(0.01);
            }

            let reproduction_threshold = organism.reproduction_threshold
                * (1.0 + rng.0.gen_range(-mutation_factor..mutation_factor));

            let mutated_size =
                organism.size * (1.0 + rng.0.gen_range(-mutation_factor..mutation_factor));
            let size = mutated_size.max(0.1);
            let mutated_speed =
                organism.speed * (1.1 + rng.0.gen_range(-mutation_factor..mutation_factor));
            let penalty = size * 0.1;
            let speed = (mutated_speed - penalty).max(0.1);

            let mutated_cooldown = (config.organism.organism_reproduction_cooldown
                * (1.0 + rng.0.gen_range(-mutation_factor..mutation_factor)))
            .max(1.0);

            let child = Organism {
                energy: organism.energy / 2.0,
                speed,
                size,
                reproduction_threshold,
                biome_tolerance,
                reproduction_cooldown: mutated_cooldown,
            };

            let x_offset = rng.0.gen_range(-1..=1);
            let y_offset = rng.0.gen_range(-1..=1);

            let child_position = Position {
                x: (position.x as isize + x_offset).clamp(0, world.width as isize - 1) as usize,
                y: (position.y as isize + y_offset).clamp(0, world.height as isize - 1) as usize,
            };

            let child_seed: u64 = rng.0.gen();
            commands.spawn((
                child,
                child_position,
                EntityRng(SmallRng::seed_from_u64(child_seed)),
            ));

            organism.energy /= 2.0;
            organism.reproduction_cooldown = config.organism.organism_reproduction_cooldown;
        }
    }
}

fn rebuild_spatial_index(
    mut index: ResMut<SpatialIndex>,
    query: Query<(Entity, &Position), With<Organism>>,
) {
    index.clear();
    for (entity, position) in query.iter() {
        index.insert(position.x, position.y, entity);
    }
}

fn rebuild_predator_spatial_index(
    mut index: ResMut<PredatorSpatialIndex>,
    query: Query<(Entity, &Position), With<Predator>>,
) {
    index.0.clear();
    for (entity, position) in query.iter() {
        index.0.insert(position.x, position.y, entity);
    }
}

fn hunting(
    mut commands: Commands,
    mut predator_query: Query<(&mut Predator, &Position)>,
    mut organism_query: Query<&mut Organism>,
    index: Res<SpatialIndex>,
    config: Res<Config>,
    mut eaten: Local<HashSet<Entity>>,
) {
    eaten.clear();
    for (mut predator, predator_position) in predator_query.iter_mut() {
        if predator.energy >= predator.satiation_threshold {
            continue;
        }

        let prey_entities = index.get(predator_position.x, predator_position.y);
        if !prey_entities.is_empty() {
            for &prey_entity in prey_entities {
                if eaten.contains(&prey_entity) {
                    continue;
                }
                if let Ok(mut prey) = organism_query.get_mut(prey_entity) {
                    let energy_gained = prey.size * predator.hunting_efficiency;
                    predator.energy =
                        (predator.energy + energy_gained).min(config.predator.max_predator_energy);
                    eaten.insert(prey_entity);
                    prey.energy = -1.0;
                    commands.entity(prey_entity).try_despawn_recursive();
                    break;
                }
            }
        }
    }
}

fn predator_reproduction(
    mut commands: Commands,
    mut query: Query<(&mut Predator, &Position)>,
    world: Res<World>,
    config: Res<Config>,
    mut rng: ResMut<ReproductionRng>,
    pop: Res<PopulationCount>,
) {
    let total_entities = pop.organisms + pop.predators;

    if total_entities >= config.world.max_total_entities {
        if config.world.printing {
            println!("Max entities reached, not spawning predator");
        }
        return;
    }

    for (mut predator, position) in query.iter_mut() {
        if predator.energy <= 0.0 {
            continue;
        }
        if predator.reproduction_cooldown > 0.0 {
            predator.reproduction_cooldown -= 1.0;
            continue;
        }

        if predator.energy > predator.reproduction_threshold {
            let mutation_factor = config.predator.predator_mutability;

            let mutated_size =
                predator.size * (1.0 + rng.0.gen_range(-mutation_factor..mutation_factor));
            let size = mutated_size.max(0.1);

            let mutated_speed =
                predator.speed * (1.1 + rng.0.gen_range(-mutation_factor..mutation_factor));
            let penalty = size * 0.1;
            let speed = (mutated_speed - penalty).max(0.1);

            let reproduction_cooldown = (config.predator.predator_reproduction_cooldown
                * (1.0 + rng.0.gen_range(-mutation_factor..mutation_factor)))
            .max(1.0);

            let child = Predator {
                energy: predator.energy / 2.0,
                speed,
                size,
                hunting_efficiency: predator.hunting_efficiency
                    * (1.0 + rng.0.gen_range(-mutation_factor..mutation_factor)),
                satiation_threshold: predator.satiation_threshold
                    * (1.0 + rng.0.gen_range(-mutation_factor..mutation_factor)),
                reproduction_threshold: predator.reproduction_threshold
                    * (1.0 + rng.0.gen_range(-mutation_factor..mutation_factor)),
                reproduction_cooldown,
            };

            let x_offset = rng.0.gen_range(-1..=1);
            let y_offset = rng.0.gen_range(-1..=1);

            let child_position = Position {
                x: (position.x as isize + x_offset).clamp(0, world.width as isize - 1) as usize,
                y: (position.y as isize + y_offset).clamp(0, world.height as isize - 1) as usize,
            };

            let child_seed: u64 = rng.0.gen();
            commands.spawn((
                child,
                child_position,
                EntityRng(SmallRng::seed_from_u64(child_seed)),
            ));

            predator.energy /= 2.0;
            predator.reproduction_cooldown = config.predator.predator_reproduction_cooldown;
        }
    }
}

fn organism_overcrowding(
    mut query: Query<&mut Organism>,
    index: Res<SpatialIndex>,
    config: Res<Config>,
    mut scratch: Local<Vec<(Entity, f32)>>,
) {
    let threshold = config.organism.overcrowding_threshold_for_organisms;
    for cell in index.cells.iter() {
        if cell.len() <= threshold {
            continue;
        }

        scratch.clear();
        for &entity in cell.iter() {
            if let Ok(organism) = query.get(entity) {
                scratch.push((entity, organism.energy));
            }
        }

        scratch.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let to_remove = scratch.len().saturating_sub(threshold);
        for &(entity, _) in scratch.iter().take(to_remove) {
            if let Ok(mut organism) = query.get_mut(entity) {
                organism.energy = -1.0;
                if config.world.printing {
                    println!("Organism died due to overcrowding");
                }
            }
        }
    }
}

fn predator_overcrowding(
    mut query: Query<&mut Predator>,
    index: Res<PredatorSpatialIndex>,
    config: Res<Config>,
    mut scratch: Local<Vec<(Entity, f32)>>,
) {
    let threshold = config.predator.overcrowding_threshold_for_predators;
    for cell in index.0.cells.iter() {
        if cell.len() <= threshold {
            continue;
        }

        scratch.clear();
        for &entity in cell.iter() {
            if let Ok(predator) = query.get(entity) {
                scratch.push((entity, predator.energy));
            }
        }

        scratch.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let to_remove = scratch.len().saturating_sub(threshold);
        for &(entity, _) in scratch.iter().take(to_remove) {
            if let Ok(mut predator) = query.get_mut(entity) {
                predator.energy = -1.0;
                if config.world.printing {
                    println!("Predator died due to overcrowding");
                }
            }
        }
    }
}

fn increment_generation(mut generation: ResMut<Generation>) {
    generation.0 += 1;
}

fn kill_over_limit_organisms(
    mut commands: Commands,
    organisms_query: Query<(Entity, &Organism)>,
    predators_query: Query<(Entity, &Predator)>,
    config: Res<Config>,
    mut rng: ResMut<SpawnRng>,
    pop: Res<PopulationCount>,
) {
    let limit = config.world.max_total_entities;
    let total_entities = pop.organisms + pop.predators;
    let over_limit = total_entities as i32 - limit as i32;
    if over_limit <= 0 {
        return;
    }
    let to_kill = over_limit as usize;
    let kill_prob = to_kill as f32 / total_entities as f32;
    let mut killed = 0usize;

    for (entity, _) in organisms_query.iter() {
        if killed >= to_kill {
            break;
        }
        if rng.0.gen::<f32>() < kill_prob {
            commands.entity(entity).despawn_recursive();
            killed += 1;
        }
    }
    for (entity, _) in predators_query.iter() {
        if killed >= to_kill {
            break;
        }
        if rng.0.gen::<f32>() < kill_prob {
            commands.entity(entity).despawn_recursive();
            killed += 1;
        }
    }
}

fn run_for_x_generations(
    generation: Res<Generation>,
    config: Res<Config>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if let Some(limit) = config.world.generation_limit {
        if generation.0 >= limit {
            next_state.set(AppState::Finished);
        }
    }
}

fn print_simulation_progress(
    generation: Res<Generation>,
    config: Res<Config>,
    app_state: Res<State<AppState>>,
    pop: Res<PopulationCount>,
) {
    if !config.world.printing {
        return;
    }
    if app_state.get() == &AppState::Simulate {
        let organisms_count = pop.organisms;
        let predators_count = pop.predators;
        let total_entities = organisms_count + predators_count;
        println!(
            "Generation: {} / {}, Total entities: {}, Organisms: {}, Predators: {}",
            generation.0,
            config.world.generation_limit.unwrap_or(0),
            total_entities,
            organisms_count,
            predators_count
        );
    }
}

fn exit_app(mut exit: EventWriter<AppExit>, app_state: Res<State<AppState>>) {
    if app_state.get() == &AppState::Finished {
        exit.send(AppExit::Success);
    }
}

use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use serde::Serialize;

use crate::components::{Organism, Position, Predator};
use crate::plugins::simulation::SimulationSet;
use crate::resources::{AppState, Biome, Config, FoodGrid, Generation, World};

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, initialize_log_file)
            .add_systems(
                Update,
                (log_world_data, log_preprocessed_world_data)
                    .run_if(in_state(AppState::Simulate))
                    .after(SimulationSet),
            )
            .add_systems(OnEnter(AppState::Finished), flush_log);
    }
}

enum LogTarget {
    World,
    Summary,
}

struct LogMessage {
    target: LogTarget,
    line: String,
}

#[derive(Resource)]
struct LogWriter {
    sender: Mutex<Option<Sender<LogMessage>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl LogWriter {
    fn send(&self, msg: LogMessage) {
        if let Some(ref tx) = *self.sender.lock().unwrap() {
            tx.send(msg).ok();
        }
    }
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
struct ExportData<'a> {
    config: &'a Config,
    organisms: Vec<OrganismWithPosition>,
    predators: Vec<PredatorWithPosition>,
    world: &'a World,
    food: &'a [f32],
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

fn initialize_log_file(mut commands: Commands, config: Res<Config>) {
    if !config.logging.log_data {
        return;
    }

    let (tx, rx) = mpsc::channel::<LogMessage>();

    let handle = thread::spawn(move || {
        let mut world_file =
            BufWriter::new(File::create("world_data.jsonl").expect("Failed to create world log"));
        let mut summary_file = BufWriter::new(
            File::create("summary_data.jsonl").expect("Failed to create summary log"),
        );

        while let Ok(msg) = rx.recv() {
            let file = match msg.target {
                LogTarget::World => &mut world_file,
                LogTarget::Summary => &mut summary_file,
            };
            writeln!(file, "{}", msg.line).expect("Failed to write log line");
        }

        world_file.flush().expect("Failed to flush world log");
        summary_file.flush().expect("Failed to flush summary log");
    });

    commands.insert_resource(LogWriter {
        sender: Mutex::new(Some(tx)),
        handle: Mutex::new(Some(handle)),
    });
}

fn log_world_data(
    config: Res<Config>,
    world: Res<World>,
    food_grid: Res<FoodGrid>,
    generation: Res<Generation>,
    organisms_query: Query<(&Organism, &Position)>,
    predators_query: Query<(&Predator, &Position)>,
    log_writer: Option<Res<LogWriter>>,
) {
    let Some(log_writer) = log_writer else { return };
    let interval = config.logging.log_interval.max(1);
    if generation.0 % interval != 0 {
        return;
    }

    let organisms = organisms_query
        .iter()
        .map(|(organism, position)| OrganismWithPosition {
            organism: organism.clone(),
            position: *position,
        })
        .collect::<Vec<_>>();

    let predators = predators_query
        .iter()
        .map(|(predator, position)| PredatorWithPosition {
            predator: *predator,
            position: *position,
        })
        .collect::<Vec<_>>();

    let export = ExportData {
        config: &config,
        organisms,
        predators,
        world: &world,
        food: &food_grid.0,
        generation: generation.0,
    };

    let line = serde_json::to_string(&export).expect("Failed to serialize world data");
    log_writer.send(LogMessage {
        target: LogTarget::World,
        line,
    });
}

fn log_preprocessed_world_data(
    config: Res<Config>,
    food_grid: Res<FoodGrid>,
    generation: Res<Generation>,
    organisms_query: Query<(&Organism, &Position)>,
    predators_query: Query<(&Predator, &Position)>,
    log_writer: Option<Res<LogWriter>>,
) {
    let Some(log_writer) = log_writer else { return };
    let interval = config.logging.log_interval.max(1);
    if generation.0 % interval != 0 {
        return;
    }

    let mut biome_tally = HashMap::new();
    let mut organism_count = 0;
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

        for biome in [Biome::Forest, Biome::Desert, Biome::Water, Biome::Grassland] {
            *biome_tally.entry(biome).or_insert(0.0) += organism.biome_tolerance[biome.idx()];
        }
    }

    let mut predator_count = 0;
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

    let total_tiles = (config.world.width * config.world.height) as f32;
    let total_food: f32 = food_grid.0.iter().sum();

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

    let line = serde_json::to_string(&summary).expect("Failed to serialize summary data");
    log_writer.send(LogMessage {
        target: LogTarget::Summary,
        line,
    });
}

fn flush_log(log_writer: Option<Res<LogWriter>>) {
    let Some(log_writer) = log_writer else { return };
    drop(log_writer.sender.lock().unwrap().take());
    let handle = log_writer.handle.lock().unwrap().take();
    if let Some(h) = handle {
        let _ = h.join();
    }
}

use std::error::Error;
use std::fs;

use rand::prelude::*;

use crate::resources::{
    Biome, BiomeDataConfig, Config, LoggingConfig, OrganismConfig, PredatorConfig, WorldConfig,
};

pub const DIRECTIONS: [(isize, isize); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

pub fn get_biome_tolerance(tile_biome: Biome, rng: &mut impl Rng) -> [f32; 4] {
    let mut tolerances = [0.0f32; 4];
    for biome in [Biome::Forest, Biome::Desert, Biome::Water, Biome::Grassland] {
        tolerances[biome.idx()] = if biome == tile_biome {
            rng.gen_range(1.0..1.5)
        } else {
            rng.gen_range(0.1..0.8)
        };
    }
    tolerances
}

pub fn load_config() -> Result<Config, Box<dyn Error>> {
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
pub fn default_config() -> Config {
    Config {
        world: WorldConfig {
            width: 10,
            height: 10,
            seed: 0,
            headless: false,
            printing: false,
            generation_limit: None,
            max_total_entities: 1000,
            forest: BiomeDataConfig {
                food_availability: 1.0,
                max_food_availability: 100.0,
            },
            desert: BiomeDataConfig {
                food_availability: 1.0,
                max_food_availability: 100.0,
            },
            water: BiomeDataConfig {
                food_availability: 1.0,
                max_food_availability: 100.0,
            },
            grassland: BiomeDataConfig {
                food_availability: 1.0,
                max_food_availability: 100.0,
            },
        },
        organism: OrganismConfig {
            initial_organisms: 10,
            initial_organism_energy: 100.0,
            initial_organism_speed: 1.0,
            initial_organism_size: 1.0,
            initial_organism_reproduction_threshold: 100.0,
            organism_mutability: 0.1,
            overcrowding_threshold_for_organisms: 10,
            organism_reproduction_cooldown: 0.5,
        },
        predator: PredatorConfig {
            initial_predators: 1,
            initial_predator_energy: 100.0,
            initial_predator_speed: 1.0,
            initial_predator_size: 1.0,
            initial_predator_reproduction_threshold: 100.0,
            initial_predator_hunting_efficiency: 1.0,
            initial_predator_satiation_threshold: 100.0,
            predator_mutability: 0.1,
            overcrowding_threshold_for_predators: 10,
            max_predator_energy: 1500.0,
            predator_energy_decay_rate: 0.5,
            predator_reproduction_cooldown: 0.5,
            predator_seek_radius: 3,
        },
        logging: LoggingConfig {
            log_data: false,
            log_interval: 1,
        },
    }
}

pub fn get_config() -> Config {
    #[cfg(target_arch = "wasm32")]
    let config = default_config();
    #[cfg(not(target_arch = "wasm32"))]
    let config = load_config().unwrap_or_else(|err| {
        eprintln!("Failed to load config: {}. Using default config.", err);
        default_config()
    });

    config
}

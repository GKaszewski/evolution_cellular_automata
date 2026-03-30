use std::fmt::Display;

use bevy::prelude::*;
use noise::NoiseFn;
use noise::Perlin;
use rand::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Simulate,
    Finished,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct BiomeDataConfig {
    pub food_availability: f32,
    pub max_food_availability: f32,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct WorldConfig {
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub headless: bool,
    pub printing: bool,
    pub generation_limit: Option<usize>,
    pub max_total_entities: usize,
    pub forest: BiomeDataConfig,
    pub desert: BiomeDataConfig,
    pub water: BiomeDataConfig,
    pub grassland: BiomeDataConfig,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct OrganismConfig {
    pub initial_organisms: usize,
    pub initial_organism_energy: f32,
    pub initial_organism_speed: f32,
    pub initial_organism_size: f32,
    pub initial_organism_reproduction_threshold: f32,
    pub organism_mutability: f32,
    pub overcrowding_threshold_for_organisms: usize,
    pub organism_reproduction_cooldown: f32,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct PredatorConfig {
    pub initial_predators: usize,
    pub initial_predator_energy: f32,
    pub initial_predator_speed: f32,
    pub initial_predator_size: f32,
    pub initial_predator_reproduction_threshold: f32,
    pub initial_predator_hunting_efficiency: f32,
    pub initial_predator_satiation_threshold: f32,
    pub predator_mutability: f32,
    pub overcrowding_threshold_for_predators: usize,
    pub max_predator_energy: f32,
    pub predator_energy_decay_rate: f32,
    pub predator_reproduction_cooldown: f32,
    #[serde(default = "default_predator_seek_radius")]
    pub predator_seek_radius: usize,
}

fn default_predator_seek_radius() -> usize {
    3
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct LoggingConfig {
    pub log_data: bool,
    #[serde(default = "default_log_interval")]
    pub log_interval: usize,
}

fn default_log_interval() -> usize {
    1
}

#[derive(Deserialize, Debug, Resource, Serialize, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub world: WorldConfig,
    #[serde(flatten)]
    pub organism: OrganismConfig,
    #[serde(flatten)]
    pub predator: PredatorConfig,
    #[serde(flatten)]
    pub logging: LoggingConfig,
}

#[derive(Resource, Default)]
pub struct PopulationCount {
    pub organisms: usize,
    pub predators: usize,
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

impl Biome {
    pub fn idx(self) -> usize {
        self as usize
        // Forest=0, Desert=1, Water=2, Grassland=3
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Tile {
    pub biome: Biome,
    pub temperature: f32,
    pub humidity: f32,
}

#[derive(Debug, Resource, Serialize, Clone)]
pub struct World {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<Tile>,
}

impl World {
    pub fn new(width: usize, height: usize, random_seed: u64) -> (Self, FoodGrid) {
        let mut rng = StdRng::seed_from_u64(random_seed);
        let seed = rng.gen::<u32>();

        let perlin = Perlin::new(seed);
        let scale = 10.0;

        let mut grid = Vec::with_capacity(width * height);
        let mut food = Vec::with_capacity(width * height);

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

                grid.push(Tile {
                    biome,
                    temperature: 20.0,
                    humidity: 0.5,
                });
                food.push(rng.gen_range(1.0..100.0_f32));
            }
        }

        (
            Self {
                width,
                height,
                grid,
            },
            FoodGrid(food),
        )
    }

    #[inline]
    pub fn tile(&self, x: usize, y: usize) -> &Tile {
        &self.grid[y * self.width + x]
    }
}

impl Default for World {
    fn default() -> Self {
        World::new(10, 10, 0).0
    }
}

#[derive(Default, Resource, Serialize)]
pub struct Generation(pub usize);

#[derive(Resource)]
pub struct ReproductionRng(pub SmallRng);

#[derive(Resource)]
pub struct SpawnRng(pub SmallRng);

#[derive(Resource)]
pub struct SpatialIndex {
    pub cells: Vec<Vec<Entity>>,
    pub width: usize,
}

impl SpatialIndex {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![Vec::new(); width * height],
            width,
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> &[Entity] {
        &self.cells[y * self.width + x]
    }

    #[inline]
    pub fn insert(&mut self, x: usize, y: usize, entity: Entity) {
        self.cells[y * self.width + x].push(entity);
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
    }
}

#[derive(Resource)]
pub struct PredatorSpatialIndex(pub SpatialIndex);

impl PredatorSpatialIndex {
    pub fn new(width: usize, height: usize) -> Self {
        Self(SpatialIndex::new(width, height))
    }
}

#[derive(Resource, Debug, Clone, Serialize)]
pub struct FoodGrid(pub Vec<f32>);

pub const TILE_SIZE_IN_PIXELS: f32 = 32.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predator_spatial_index_insert_and_get() {
        let mut idx = SpatialIndex::new(5, 5);
        let e = Entity::from_raw(42);
        idx.insert(2, 3, e);
        assert_eq!(idx.get(2, 3), &[e]);
        assert_eq!(idx.get(0, 0), &[] as &[Entity]);
    }

    #[test]
    fn predator_spatial_index_clear() {
        let mut idx = SpatialIndex::new(5, 5);
        let e = Entity::from_raw(1);
        idx.insert(0, 0, e);
        idx.clear();
        assert_eq!(idx.get(0, 0), &[] as &[Entity]);
    }
}

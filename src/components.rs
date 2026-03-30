use bevy::prelude::*;
use rand::rngs::SmallRng;
use serde::Serialize;

use crate::resources::Biome;

#[derive(Component, Serialize, Clone)]
pub struct Organism {
    pub energy: f32,
    pub speed: f32,
    pub size: f32,
    pub reproduction_threshold: f32,
    pub reproduction_cooldown: f32,
    pub biome_tolerance: [f32; 4],
}

#[derive(Component, Serialize, Copy, Clone)]
pub struct Predator {
    pub energy: f32,
    pub speed: f32,
    pub size: f32,
    pub reproduction_threshold: f32,
    pub hunting_efficiency: f32,
    pub satiation_threshold: f32,
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

#[derive(Component)]
pub struct EntityRng(pub SmallRng);

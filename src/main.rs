use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use evolution::plugins::{LoggingPlugin, RenderingPlugin, SimulationPlugin};
use evolution::resources::{ReproductionRng, SpawnRng, World};
use evolution::*;
use rand::prelude::*;

fn main() {
    let config = get_config();

    println!("{:?}", config);

    let headless = config.world.headless;
    let mut app = App::new();

    match headless {
        true => {
            app.add_plugins((MinimalPlugins, StatesPlugin));
        }
        false => {
            app.add_plugins(DefaultPlugins);
        }
    }

    let seed = config.world.seed;
    let mut base_rng = StdRng::seed_from_u64(seed);
    let reproduction_seed: u64 = base_rng.gen();
    let spawn_seed: u64 = base_rng.gen();

    let (world, food_grid) = World::new(config.world.width, config.world.height, seed);
    app.insert_resource(world)
        .insert_resource(food_grid)
        .insert_resource(ReproductionRng(SmallRng::seed_from_u64(reproduction_seed)))
        .insert_resource(SpawnRng(SmallRng::seed_from_u64(spawn_seed)))
        .insert_resource(SpatialIndex::new(config.world.width, config.world.height))
        .insert_resource(PredatorSpatialIndex::new(
            config.world.width,
            config.world.height,
        ))
        .insert_resource(PopulationCount::default())
        .insert_resource(config)
        .insert_resource(Generation(0))
        .init_state::<AppState>()
        .add_plugins((SimulationPlugin, RenderingPlugin, LoggingPlugin))
        .run();
}

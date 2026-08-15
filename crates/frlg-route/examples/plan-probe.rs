//! Ask the path planner for one leg directly: plan-probe <group.num> <sx>
//! <sy> <tx> <ty>. Prints the plan's steps and model cost -- for checking
//! a flagged dogleg against what the model actually believes.

use frlg_route::plan::{plan, PlanRequest};
use frlg_route::world::World;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (map_id, sx, sy, tx, ty) = (
        args[1].clone(),
        args[2].parse::<i16>()?,
        args[3].parse::<i16>()?,
        args[4].parse::<i16>()?,
        args[5].parse::<i16>()?,
    );
    let (g, n) = map_id.split_once('.').ok_or("map as group.num")?;
    let mut world = World::load()?;
    let map = world.map((g.parse()?, n.parse()?))?;
    let req = PlanRequest {
        map,
        wild: None,
        start: (sx, sy),
        wild_data: frlg_route::observe::WildData {
            rng_state: 1,
            prev_behavior: 0,
            rate_buff: 0,
            steps_since: 0,
        },
        targets: vec![(tx, ty)],
        blocked: Default::default(),
        encounter_cost: frlg_route::plan::ENCOUNTER_COST,
        test_bias: 0,
    };
    match plan(&req) {
        None => println!("no plan"),
        Some((steps, cost)) => {
            println!("{} steps, model cost {cost}", steps.len());
            for s in &steps {
                println!("  {:?} {:?}", s.to, s.kind);
            }
        }
    }
    Ok(())
}

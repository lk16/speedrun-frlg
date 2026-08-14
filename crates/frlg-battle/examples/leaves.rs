//! Print every engine leaf for the given plans on the committed anchor.
//!
//!     cargo run --release -p frlg-battle --example leaves -- "4,3,3,0" ...

use frlg_battle::engine::simulate;
use frlg_battle::Mon;
use frlg_rng::Rng;

fn main() {
    let us = Mon {
        hp: 20,
        max_hp: 20,
        attack: 11,
        defense: 10,
        speed: 11,
        level: 5,
        atk_stage: 6,
        def_stage: 6,
    };
    let rival = Mon {
        hp: 18,
        max_hp: 18,
        attack: 11,
        defense: 9,
        speed: 9,
        level: 5,
        atk_stage: 6,
        def_stage: 6,
    };
    for arg in std::env::args().skip(1) {
        let plan: Vec<u32> = arg.split(',').map(|d| d.parse().expect("delay")).collect();
        println!("plan {plan:?}:");
        for leaf in simulate(&plan, Rng(0xed94271d), us, rival) {
            println!("  gates {:?} -> {:?}", leaf.commit_durs, leaf.result);
        }
    }
}

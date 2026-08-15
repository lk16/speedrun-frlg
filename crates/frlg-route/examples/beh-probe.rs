fn main() {
    let mut w = frlg_route::world::World::load().unwrap();
    let m = w.map((4, 3)).unwrap();
    for x in 5..=7 {
        let t = m.tile(x, 12).unwrap();
        println!(
            "({x},12) behavior {:#04x} collision {}",
            t.behavior, t.collision
        );
    }
}

//! Unit tests of the walker's object model (review round 1, R1).
use super::walk::ObjInfo;

fn obj(movement: &str, rx: i16, ry: i16) -> ObjInfo {
    ObjInfo {
        x: 40,
        y: 11,
        local_id: 2,
        movement: movement.into(),
        script: String::new(),
        graphics: String::new(),
        flag: 0,
        trainer: true,
        sight: 3,
        beaten: 0,
        range_x: rx,
        range_y: ry,
    }
}

#[test]
fn facings_and_boxes_follow_the_movement_type() {
    // Lass Robin (`decompiled/data/maps/Route3/map.json`): wanders up and down, range 1 x 1.
    let robin = obj("MOVEMENT_TYPE_WANDER_UP_AND_DOWN", 1, 1);
    assert_eq!(robin.facings(), vec![(0, -1), (0, 1)]);
    assert_eq!(robin.offsets(), vec![(0, -1), (0, 0), (0, 1)]);
    // Youngster Ben: faces down and never moves (his 1 x 1 range is unused).
    let ben = obj("MOVEMENT_TYPE_FACE_DOWN", 1, 1);
    assert_eq!(ben.facings(), vec![(0, 1)]);
    assert_eq!(ben.offsets(), vec![(0, 0)]);
    assert_eq!(obj("MOVEMENT_TYPE_LOOK_AROUND", 1, 1).facings().len(), 4);
    assert_eq!(
        obj("MOVEMENT_TYPE_FACE_DOWN_AND_LEFT", 0, 0).facings(),
        vec![(0, 1), (-1, 0)]
    );
    assert_eq!(obj("MOVEMENT_TYPE_WANDER_AROUND", 1, 2).offsets().len(), 15);
}

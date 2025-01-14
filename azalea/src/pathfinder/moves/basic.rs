use std::f32::consts::SQRT_2;

use azalea_client::{SprintDirection, StartSprintEvent, StartWalkEvent, WalkDirection};
use azalea_core::{direction::CardinalDirection, position::BlockPos};
use azalea_world::Instance;

use crate::{
    pathfinder::{astar, costs::*},
    JumpEvent, LookAtEvent,
};

use super::{
    default_is_reached, fall_distance, is_block_passable, is_passable, is_standable, Edge,
    ExecuteCtx, IsReachedCtx, MoveData,
};

pub fn basic_move(world: &Instance, node: BlockPos) -> Vec<Edge> {
    let mut edges = Vec::new();
    edges.extend(forward_move(world, node));
    edges.extend(ascend_move(world, node));
    edges.extend(descend_move(world, node));
    edges.extend(diagonal_move(world, node));
    edges
}

fn forward_move(world: &Instance, pos: BlockPos) -> Vec<Edge> {
    let mut edges = Vec::new();
    for dir in CardinalDirection::iter() {
        let offset = BlockPos::new(dir.x(), 0, dir.z());

        if !is_standable(&(pos + offset), world) {
            continue;
        }

        let cost = SPRINT_ONE_BLOCK_COST;

        edges.push(Edge {
            movement: astar::Movement {
                target: pos + offset,
                data: MoveData {
                    execute: &execute_forward_move,
                    is_reached: &default_is_reached,
                },
            },
            cost,
        })
    }

    edges
}

fn execute_forward_move(
    ExecuteCtx {
        entity,
        target,
        look_at_events,
        sprint_events,
        ..
    }: ExecuteCtx,
) {
    let center = target.center();
    look_at_events.send(LookAtEvent {
        entity,
        position: center,
    });
    sprint_events.send(StartSprintEvent {
        entity,
        direction: SprintDirection::Forward,
    });
}

fn ascend_move(world: &Instance, pos: BlockPos) -> Vec<Edge> {
    let mut edges = Vec::new();
    for dir in CardinalDirection::iter() {
        let offset = BlockPos::new(dir.x(), 1, dir.z());

        if !is_block_passable(&pos.up(2), world) {
            continue;
        }
        if !is_standable(&(pos + offset), world) {
            continue;
        }

        let cost = SPRINT_ONE_BLOCK_COST + *JUMP_ONE_BLOCK_COST;

        edges.push(Edge {
            movement: astar::Movement {
                target: pos + offset,
                data: MoveData {
                    execute: &execute_ascend_move,
                    is_reached: &ascend_is_reached,
                },
            },
            cost,
        })
    }
    edges
}
fn execute_ascend_move(
    ExecuteCtx {
        entity,
        position,
        target,
        start,
        look_at_events,
        walk_events,
        jump_events,
        physics,
        ..
    }: ExecuteCtx,
) {
    let target_center = target.center();

    look_at_events.send(LookAtEvent {
        entity,
        position: target_center,
    });
    walk_events.send(StartWalkEvent {
        entity,
        direction: WalkDirection::Forward,
    });

    // these checks are to make sure we don't fall if our velocity is too high in
    // the wrong direction

    let x_axis = (start.x - target.x).abs(); // either 0 or 1
    let z_axis = (start.z - target.z).abs(); // either 0 or 1

    let flat_distance_to_next = x_axis as f64 * (target_center.x - position.x)
        + z_axis as f64 * (target_center.z - position.z);
    let side_distance = z_axis as f64 * (target_center.x - position.x).abs()
        + x_axis as f64 * (target_center.z - position.z).abs();

    let lateral_motion = x_axis as f64 * physics.delta.z + z_axis as f64 * physics.delta.x;
    if lateral_motion > 0.1 {
        return;
    }

    if flat_distance_to_next > 1.2 || side_distance > 0.2 {
        return;
    }

    jump_events.send(JumpEvent { entity });
}
#[must_use]
pub fn ascend_is_reached(
    IsReachedCtx {
        position, target, ..
    }: IsReachedCtx,
) -> bool {
    BlockPos::from(position) == target || BlockPos::from(position) == target.down(1)
}

fn descend_move(world: &Instance, pos: BlockPos) -> Vec<Edge> {
    let mut edges = Vec::new();
    for dir in CardinalDirection::iter() {
        let new_horizontal_position = pos + BlockPos::new(dir.x(), 0, dir.z());
        let fall_distance = fall_distance(&new_horizontal_position, world);
        if fall_distance == 0 || fall_distance > 3 {
            continue;
        }
        let new_position = new_horizontal_position.down(fall_distance as i32);

        // check whether 3 blocks vertically forward are passable
        if !is_passable(&new_horizontal_position, world) {
            continue;
        }
        // check whether we can stand on the target position
        if !is_standable(&new_position, world) {
            continue;
        }

        let cost = SPRINT_ONE_BLOCK_COST + FALL_ONE_BLOCK_COST * fall_distance as f32;

        edges.push(Edge {
            movement: astar::Movement {
                target: new_position,
                data: MoveData {
                    execute: &execute_descend_move,
                    is_reached: &descend_is_reached,
                },
            },
            cost,
        })
    }
    edges
}
fn execute_descend_move(
    ExecuteCtx {
        entity,
        target,
        start,
        look_at_events,
        sprint_events,
        position,
        ..
    }: ExecuteCtx,
) {
    let center = target.center();
    let horizontal_distance_from_target = (center - position).horizontal_distance_sqr().sqrt();
    let horizontal_distance_from_start =
        (start.center() - position).horizontal_distance_sqr().sqrt();

    let dest_ahead = BlockPos::new(
        start.x + (target.x - start.x) * 2,
        target.y,
        start.z + (target.z - start.z) * 2,
    );

    if BlockPos::from(position) != target || horizontal_distance_from_target > 0.25 {
        // if we're only falling one block then it's fine to try to overshoot
        if horizontal_distance_from_start < 1.25 || start.y - target.y == 1 {
            // this basically just exists to avoid doing spins while we're falling
            look_at_events.send(LookAtEvent {
                entity,
                position: dest_ahead.center(),
            });
            sprint_events.send(StartSprintEvent {
                entity,
                direction: SprintDirection::Forward,
            });
        } else {
            look_at_events.send(LookAtEvent {
                entity,
                position: center,
            });
            sprint_events.send(StartSprintEvent {
                entity,
                direction: SprintDirection::Forward,
            });
        }
    }
}
#[must_use]
pub fn descend_is_reached(
    IsReachedCtx {
        target,
        start,
        position,
        ..
    }: IsReachedCtx,
) -> bool {
    let dest_ahead = BlockPos::new(
        start.x + (target.x - start.x) * 2,
        target.y,
        start.z + (target.z - start.z) * 2,
    );

    (BlockPos::from(position) == target || BlockPos::from(position) == dest_ahead)
        && (position.y - target.y as f64) < 0.5
}

fn diagonal_move(world: &Instance, pos: BlockPos) -> Vec<Edge> {
    let mut edges = Vec::new();
    for dir in CardinalDirection::iter() {
        let right = dir.right();
        let offset = BlockPos::new(dir.x() + right.x(), 0, dir.z() + right.z());

        if !is_passable(
            &BlockPos::new(pos.x + dir.x(), pos.y, pos.z + dir.z()),
            world,
        ) && !is_passable(
            &BlockPos::new(pos.x + dir.right().x(), pos.y, pos.z + dir.right().z()),
            world,
        ) {
            continue;
        }
        if !is_standable(&(pos + offset), world) {
            continue;
        }
        // +0.001 so it doesn't unnecessarily go diagonal sometimes
        let cost = SPRINT_ONE_BLOCK_COST * SQRT_2 + 0.001;

        edges.push(Edge {
            movement: astar::Movement {
                target: pos + offset,
                data: MoveData {
                    execute: &execute_diagonal_move,
                    is_reached: &default_is_reached,
                },
            },
            cost,
        })
    }
    edges
}
fn execute_diagonal_move(
    ExecuteCtx {
        entity,
        target,
        look_at_events,
        sprint_events,
        ..
    }: ExecuteCtx,
) {
    let center = target.center();
    look_at_events.send(LookAtEvent {
        entity,
        position: center,
    });
    sprint_events.send(StartSprintEvent {
        entity,
        direction: SprintDirection::Forward,
    });
}

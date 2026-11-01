//! Behavioural tests for the rules recovered from `Assembly-CSharp.dll`.
//!
//! Each test names the decompiled method it pins down.

use glam::Vec3;
use granny_rs::game::{AiState, Difficulty, Door, DoorState, Item, World};

fn world() -> World {
    World::new(Difficulty::Normal, Vec3::ZERO)
}

/// `EnemyAIGranny.GrannyDecisions` (0x145b020) returns immediately while any
/// of the six busy flags is set, so the AI must not transition.
#[test]
fn granny_decisions_early_out_on_busy_flags() {
    let mut w = world();
    w.granny.pos = Vec3::new(0.5, 0.0, 0.0); // inside ATTACK_RANGE
    w.granny.state = AiState::Patrol;

    w.granny.busy.frozen = true;
    w.granny_decisions(0.016);
    assert_eq!(
        w.granny.state,
        AiState::Patrol,
        "frozen must suppress decisions"
    );
    assert!(!w.caught);

    w.granny.busy.frozen = false;
    w.granny_decisions(0.016);
    assert_eq!(
        w.granny.state,
        AiState::Attack,
        "close + unblocked => attack"
    );
    assert!(w.caught);
}

/// The six flags are exactly those read by the original early-out.
#[test]
fn ai_busy_any_covers_all_six_flags() {
    let mut w = world();
    for set in 0..6 {
        w.granny.busy = Default::default();
        match set {
            0 => w.granny.busy.hit_by_arrow = true,
            1 => w.granny.busy.hit_by_gun = true,
            2 => w.granny.busy.hit_by_car = true,
            3 => w.granny.busy.hit_by_pepper = true,
            4 => w.granny.busy.frozen = true,
            _ => w.granny.busy.dropping_trap = true,
        }
        assert!(w.granny.busy.any(), "flag {set} must count as busy");
    }
    w.granny.busy = Default::default();
    assert!(!w.granny.busy.any());
}

/// `DifficultyData`: Extreme is fastest, Easy slowest.
#[test]
fn difficulty_orders_granny_speed() {
    assert!(Difficulty::Easy.granny_speed() < Difficulty::Normal.granny_speed());
    assert!(Difficulty::Normal.granny_speed() < Difficulty::Hard.granny_speed());
    assert!(Difficulty::Hard.granny_speed() < Difficulty::Extreme.granny_speed());
    assert_eq!(Difficulty::Easy.days(), 6);
    assert_eq!(Difficulty::Normal.days(), 5);
}

/// `StartNewDay` advances the day and ends the run on the final day.
#[test]
fn next_day_ends_on_last_day() {
    let mut w = World::new(Difficulty::Normal, Vec3::ZERO);
    for _ in 0..4 {
        w.next_day();
    }
    assert_eq!(w.day, 5);
    assert!(!w.escaped);
    w.next_day();
    assert!(w.escaped, "surviving the final day must end the run");
}

/// `OpenCloseDoors` state machine.
#[test]
fn door_state_machine() {
    let mut d = Door::new(Vec3::ZERO);
    assert_eq!(d.state, DoorState::Closed);
    assert!(d.blocked());

    d.interact();
    assert_eq!(d.state, DoorState::Opening);
    d.update(1.0);
    assert_eq!(d.state, DoorState::Open);
    assert!(!d.blocked());

    d.interact();
    assert_eq!(d.state, DoorState::Closing);
    d.update(1.0);
    assert_eq!(d.state, DoorState::Closed);
}

/// A locked door never opens.
#[test]
fn locked_door_refuses_to_open() {
    let mut d = Door::new(Vec3::ZERO);
    d.locked = true;
    d.interact();
    assert_eq!(d.state, DoorState::Locked);
    assert!(d.blocked());
}

/// `InventoryController`: an item is only stored once.
#[test]
fn inventory_dedupes() {
    let mut w = world();
    w.give(Item::Key);
    w.give(Item::Key);
    assert_eq!(w.inventory.len(), 1);
    assert!(w.has(Item::Key));
    assert!(!w.has(Item::Hammer));
}

/// Pickups are collected on proximity and cannot be taken twice.
#[test]
fn pickup_collects_once() {
    let mut w = world();
    w.pickups.push(granny_rs::game::Pickup {
        pos: Vec3::new(0.5, 0.0, 0.0),
        item: Item::Hammer,
        taken: false,
    });
    w.tick(0.016, glam::Vec2::ZERO, false);
    assert!(w.has(Item::Hammer));
    assert!(w.pickups[0].taken);

    w.inventory.clear();
    w.tick(0.016, glam::Vec2::ZERO, false);
    assert!(!w.has(Item::Hammer), "a taken pickup must not respawn");
}

/// `FPSControllerNEW`: crouching is slower and quieter than running.
#[test]
fn movement_speed_and_noise() {
    let mut w = world();
    w.granny.pos = Vec3::new(500.0, 0.0, 500.0); // out of range

    w.player.crouch = false;
    w.tick(0.1, glam::Vec2::new(0.0, 1.0), false);
    let walk_noise = w.player.noise;
    let walk_step = w.player.pos.z;

    let mut c = world();
    c.granny.pos = Vec3::new(500.0, 0.0, 500.0);
    c.player.crouch = true;
    c.tick(0.1, glam::Vec2::new(0.0, 1.0), false);

    assert!(c.player.pos.z < walk_step, "crouch must be slower");
    assert!(c.player.noise < walk_noise, "crouch must be quieter");
}

/// Granny follows a visible player and patrols otherwise.
#[test]
fn granny_sees_player_and_follows() {
    let mut w = world();
    w.granny.pos = Vec3::new(0.0, 0.0, 8.0);
    w.player.pos = Vec3::ZERO; // in front (+z looking back at granny)
    w.granny_decisions(0.016);
    assert!(matches!(w.granny.state, AiState::Follow | AiState::Attack));
}

/// Day timer expiry rolls the day over.
#[test]
fn day_timer_rolls_over() {
    let mut w = world();
    w.granny.pos = Vec3::new(900.0, 0.0, 900.0);
    w.day_time = granny_rs::game::DAY_LEN - 0.01;
    w.tick(0.02, glam::Vec2::ZERO, false);
    assert_eq!(w.day, 2);
}

/// `EnemyAIGranny.followPlayer` (0x145bb60) checks the six `GrannyDecisions`
/// flags plus two more at +0x214/+0x215 before moving.
#[test]
fn follow_player_respects_extra_suppression_flags() {
    let mut w = world();
    w.granny.state = AiState::Follow;
    w.granny.pos = Vec3::ZERO;
    w.player.pos = Vec3::new(5.0, 0.0, 0.0);

    w.granny.busy.suppress_a = true;
    let before = w.granny.pos;
    w.tick(0.1, glam::Vec2::ZERO, false);
    assert_eq!(w.granny.pos, before, "suppress_a must freeze pursuit");

    w.granny.busy.suppress_a = false;
    w.granny.busy.suppress_b = true;
    w.tick(0.1, glam::Vec2::ZERO, false);
    assert_eq!(w.granny.pos, before, "suppress_b must freeze pursuit");

    w.granny.busy.suppress_b = false;
    w.tick(0.1, glam::Vec2::ZERO, false);
    assert_ne!(w.granny.pos, before, "unblocked pursuit must move");
}

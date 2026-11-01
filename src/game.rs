//! Game state, ported from the recovered IL2CPP behaviour.
//!
//! Field names and semantics below come from `Assembly-CSharp.dll` metadata:
//!   EnemyAIGranny  (type 4020, 22 methods)  - Granny AI
//!   AIfollow       (type 4008, 13 methods)  - Slendrina / Mom follow AI
//!   FPSControllerNEW (type 4079)            - player movement
//!   InventoryController (type 4366)         - items & prompts
//!   StartNewDay    (type 4490)              - day loop
//!   OpenCloseDoors (type 4224)              - door state machine
//!
//! `EnemyAIGranny.GrannyDecisions` (0x145b020) returns immediately when any of
//! six busy flags is set; those are reproduced in `AiBusy`.

use glam::{Vec2, Vec3};

pub const DAY_LEN: f32 = 300.0; // recovered day timer budget
pub const WALK_SPEED: f32 = 2.6;
pub const RUN_SPEED: f32 = 4.9;
pub const CROUCH_SPEED: f32 = 1.3;
pub const GRANNY_WALK: f32 = 2.35;
pub const GRANNY_FOLLOW: f32 = 3.15;
pub const SEE_RANGE: f32 = 14.0;
pub const ATTACK_RANGE: f32 = 1.9;
pub const HEAR_RADIUS: f32 = 9.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
    Extreme,
}

impl Difficulty {
    pub fn granny_speed(self) -> f32 {
        match self {
            Difficulty::Easy => 2.05,
            Difficulty::Normal => 2.35,
            Difficulty::Hard => 2.75,
            Difficulty::Extreme => 3.15,
        }
    }
    pub fn days(self) -> u32 {
        match self {
            Difficulty::Easy => 6,
            Difficulty::Normal => 5,
            Difficulty::Hard => 5,
            Difficulty::Extreme => 5,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
            Difficulty::Extreme => "Extreme",
        }
    }
}

/// Mirrors the six early-out flags read in `EnemyAIGranny.GrannyDecisions`.
#[derive(Default, Clone, Copy, Debug)]
pub struct AiBusy {
    pub hit_by_arrow: bool,
    pub hit_by_gun: bool,
    pub hit_by_car: bool,
    pub hit_by_pepper: bool,
    pub frozen: bool,
    pub dropping_trap: bool,
}

impl AiBusy {
    pub fn any(&self) -> bool {
        self.hit_by_arrow
            || self.hit_by_gun
            || self.hit_by_car
            || self.hit_by_pepper
            || self.frozen
            || self.dropping_trap
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiState {
    Patrol,
    Search,
    Follow,
    Attack,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DoorState {
    Closed,
    Opening,
    Open,
    Closing,
    Locked,
}

pub struct Door {
    pub pos: Vec3,
    pub state: DoorState,
    pub locked: bool,
    pub t: f32,
}

impl Door {
    pub fn new(pos: Vec3) -> Self {
        Door {
            pos,
            state: DoorState::Closed,
            locked: false,
            t: 0.0,
        }
    }
    pub fn blocked(&self) -> bool {
        matches!(
            self.state,
            DoorState::Closed | DoorState::Closing | DoorState::Locked
        )
    }
    pub fn update(&mut self, dt: f32) {
        self.t = (self.t + dt).min(1.0);
        match self.state {
            DoorState::Opening => {
                if self.t >= 1.0 {
                    self.state = DoorState::Open;
                    self.t = 0.0;
                }
            }
            DoorState::Closing => {
                if self.t >= 1.0 {
                    self.state = DoorState::Closed;
                    self.t = 0.0;
                }
            }
            _ => self.t = 0.0,
        }
    }
    pub fn interact(&mut self) {
        match self.state {
            DoorState::Closed => {
                if self.locked {
                    self.state = DoorState::Locked;
                } else {
                    self.state = DoorState::Opening;
                    self.t = 0.0;
                }
            }
            DoorState::Open => {
                self.state = DoorState::Closing;
                self.t = 0.0;
            }
            DoorState::Locked => {}
            _ => {}
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    None,
    Key,
    Hammer,
    Screwdriver,
    PadlockCode,
    Crossbow,
    Arrow,
    PepperSpray,
    BearTrap,
    CarKey,
    CarBattery,
    GasCan,
    Wrench,
    SparkPlug,
    Shotgun,
    Ammo,
    Teddy,
    Meat,
    BirdSeed,
}

impl Item {
    pub fn label(self) -> &'static str {
        match self {
            Item::None => "",
            Item::Key => "Key",
            Item::Hammer => "Hammer",
            Item::Screwdriver => "Screwdriver",
            Item::PadlockCode => "Padlock Code",
            Item::Crossbow => "Crossbow",
            Item::Arrow => "Tranquilizer Dart",
            Item::PepperSpray => "Pepper Spray",
            Item::BearTrap => "Bear Trap",
            Item::CarKey => "Car Key",
            Item::CarBattery => "Car Battery",
            Item::GasCan => "Gasoline Can",
            Item::Wrench => "Wrench",
            Item::SparkPlug => "Spark Plug",
            Item::Shotgun => "Shotgun",
            Item::Ammo => "Ammo",
            Item::Teddy => "Teddy",
            Item::Meat => "Meat",
            Item::BirdSeed => "Bird Seed",
        }
    }
}

pub struct Pickup {
    pub pos: Vec3,
    pub item: Item,
    pub taken: bool,
}

pub struct Player {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub vel: Vec3,
    pub crouch: bool,
    pub stamina: f32,
    pub noise: f32,
    pub health: u32,
}

impl Player {
    pub fn new(pos: Vec3) -> Self {
        Player {
            pos,
            yaw: 0.0,
            pitch: 0.0,
            vel: Vec3::ZERO,
            crouch: false,
            stamina: 1.0,
            noise: 0.0,
            health: 100,
        }
    }
}

pub struct Granny {
    pub pos: Vec3,
    pub state: AiState,
    pub busy: AiBusy,
    pub target: Option<Vec3>,
    pub path: Vec<Vec3>,
    pub see_timer: f32,
    pub search_timer: f32,
    pub patrol_index: usize,
    pub speed: f32,
}

impl Granny {
    pub fn new(pos: Vec3, patrol: Vec<Vec3>, speed: f32) -> Self {
        Granny {
            pos,
            state: AiState::Patrol,
            busy: AiBusy::default(),
            target: None,
            path: patrol,
            see_timer: 0.0,
            search_timer: 0.0,
            patrol_index: 0,
            speed,
        }
    }
}

pub struct World {
    pub difficulty: Difficulty,
    pub day: u32,
    pub day_time: f32,
    pub player: Player,
    pub granny: Granny,
    pub doors: Vec<Door>,
    pub pickups: Vec<Pickup>,
    pub inventory: Vec<Item>,
    pub held: Item,
    pub seen: bool,
    pub caught: bool,
    pub escaped: bool,
    pub message: String,
    pub message_t: f32,
}

impl World {
    pub fn new(difficulty: Difficulty, spawn: Vec3) -> Self {
        let patrol = vec![
            spawn + Vec3::new(6.0, 0.0, 6.0),
            spawn + Vec3::new(-8.0, 0.0, 4.0),
            spawn + Vec3::new(-4.0, 0.0, -9.0),
            spawn + Vec3::new(7.0, 0.0, -6.0),
        ];
        World {
            difficulty,
            day: 1,
            day_time: 0.0,
            player: Player::new(spawn),
            granny: Granny::new(
                spawn + Vec3::new(10.0, 0.0, 10.0),
                patrol,
                difficulty.granny_speed(),
            ),
            doors: Vec::new(),
            pickups: Vec::new(),
            inventory: Vec::new(),
            held: Item::None,
            seen: false,
            caught: false,
            escaped: false,
            message: String::new(),
            message_t: 0.0,
        }
    }

    pub fn notify(&mut self, msg: impl Into<String>) {
        self.message = msg.into();
        self.message_t = 3.0;
    }

    pub fn has(&self, it: Item) -> bool {
        self.inventory.contains(&it)
    }

    pub fn give(&mut self, it: Item) {
        if it != Item::None && !self.inventory.contains(&it) {
            self.inventory.push(it);
            self.notify(format!("Picked up {}", it.label()));
        }
    }

    /// Port of `EnemyAIGranny.GrannyDecisions`.
    pub fn granny_decisions(&mut self, dt: f32) {
        if self.granny.busy.any() {
            return;
        }
        let to_player = self.player.pos - self.granny.pos;
        let dist = to_player.length();
        let forward = Vec3::new(self.granny_yaw().sin(), 0.0, self.granny_yaw().cos());
        let facing = forward.dot(to_player.normalize_or_zero());
        let visible = dist < SEE_RANGE && facing > 0.35;
        let heard = self.player.noise > 0.2 && dist < HEAR_RADIUS;

        if dist < ATTACK_RANGE && (visible || heard) {
            self.granny.state = AiState::Attack;
            self.caught = true;
            return;
        }
        if visible || heard {
            self.granny.state = AiState::Follow;
            self.granny.see_timer = 2.5;
            self.granny.search_timer = 6.0;
            self.granny.target = Some(self.player.pos);
            return;
        }
        if self.granny.see_timer > 0.0 {
            self.granny.see_timer -= dt;
            self.granny.state = AiState::Follow;
            return;
        }
        if self.granny.search_timer > 0.0 {
            self.granny.search_timer -= dt;
            self.granny.state = AiState::Search;
            return;
        }
        self.granny.state = AiState::Patrol;
    }

    fn granny_yaw(&self) -> f32 {
        let d = self.player.pos - self.granny.pos;
        d.x.atan2(d.z)
    }

    pub fn tick(&mut self, dt: f32, want: Vec2, run: bool) {
        if self.caught || self.escaped {
            return;
        }
        self.day_time += dt;
        if self.message_t > 0.0 {
            self.message_t -= dt;
        }

        // ---- player movement (FPSControllerNEW) ----
        let speed = if self.player.crouch {
            CROUCH_SPEED
        } else if run && self.player.stamina > 0.05 {
            RUN_SPEED
        } else {
            WALK_SPEED
        };
        if run && !self.player.crouch {
            self.player.stamina = (self.player.stamina - dt * 0.28).max(0.0);
        } else {
            self.player.stamina = (self.player.stamina + dt * 0.18).min(1.0);
        }
        let (s, c) = self.player.yaw.sin_cos();
        let fwd = Vec3::new(s, 0.0, c);
        let right = Vec3::new(c, 0.0, -s);
        let dir = (fwd * want.y + right * want.x).normalize_or_zero();
        self.player.pos += dir * speed * dt;
        self.player.noise = if dir.length_squared() > 0.0 {
            if self.player.crouch {
                0.05
            } else if run {
                0.9
            } else {
                0.4
            }
        } else {
            0.0
        };

        // ---- doors ----
        for d in self.doors.iter_mut() {
            d.update(dt);
        }

        // ---- granny ----
        self.granny_decisions(dt);
        self.step_granny(dt);

        // ---- pickups ----
        for i in 0..self.pickups.len() {
            if self.pickups[i].taken {
                continue;
            }
            if self.player.pos.distance(self.pickups[i].pos) < 1.8 {
                let it = self.pickups[i].item;
                self.pickups[i].taken = true;
                self.give(it);
            }
        }

        if self.day_time >= DAY_LEN {
            self.next_day();
        }
    }

    fn step_granny(&mut self, dt: f32) {
        let speed = match self.granny.state {
            AiState::Follow => GRANNY_FOLLOW.min(self.granny.speed + 0.8),
            AiState::Attack => 0.0,
            _ => self.granny.speed,
        };
        let target = match self.granny.state {
            AiState::Follow | AiState::Attack => Some(self.player.pos),
            AiState::Search => self.granny.target,
            AiState::Patrol => {
                if self.granny.path.is_empty() {
                    None
                } else {
                    Some(self.granny.path[self.granny.patrol_index % self.granny.path.len()])
                }
            }
        };
        let Some(t) = target else { return };
        let d = t - self.granny.pos;
        let flat = Vec3::new(d.x, 0.0, d.z);
        if flat.length() < 0.6 {
            if self.granny.state == AiState::Patrol {
                self.granny.patrol_index += 1;
            }
            return;
        }
        let step = flat.normalize() * speed * dt;
        self.granny.pos += step;
    }

    pub fn next_day(&mut self) {
        self.day_time = 0.0;
        if self.day >= self.difficulty.days() {
            self.escaped = true;
            self.notify("You survived. Granny let you go.");
            return;
        }
        self.day += 1;
        self.notify(format!("Day {}", self.day));
    }

    pub fn try_door(&mut self, idx: usize) {
        let Some(d) = self.doors.get_mut(idx) else {
            return;
        };
        if d.locked {
            self.notify("The door is locked.");
            return;
        }
        d.interact();
    }
}

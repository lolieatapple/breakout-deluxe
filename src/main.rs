use macroquad::prelude::*;

// ── Screen ──
const W: f32 = 900.0;
const H: f32 = 700.0;

// ── Paddle ──
const PADDLE_W_DEFAULT: f32 = 120.0;
const PADDLE_H: f32 = 14.0;
const PADDLE_SPEED: f32 = 600.0;
const PADDLE_Y: f32 = H - 44.0;

// ── Ball ──
const BALL_R: f32 = 7.0;
const BALL_SPEED_INIT: f32 = 340.0;
const BALL_SPEED_INC: f32 = 5.0;
const BALL_SPEED_MAX: f32 = 700.0;

// ── Bricks ──
const COLS: usize = 12;
const BW: f32 = 64.0;
const BH: f32 = 22.0;
const BGAP: f32 = 3.0;
const BTOP: f32 = 60.0;

// ── Bullets ──
const BULLET_W: f32 = 4.0;
const BULLET_H: f32 = 14.0;
const BULLET_SPEED: f32 = 600.0;
const SHOOT_COOLDOWN: f32 = 0.15;

// ── Powerups ──
const PU_SIZE: f32 = 22.0;
const PU_SPEED: f32 = 140.0;
const PU_DROP_CHANCE: f32 = 0.30;

// ── Particles ──
const LIVES_INIT: u32 = 3;
const PI: f32 = std::f32::consts::PI;

// ── Brick types ──
#[derive(Clone, Copy, PartialEq)]
enum BrickKind {
    Normal,
    Tough,       // 2 hits
    Armored,     // 3 hits
    Explosive,   // explodes neighbours
    Indestructible,
    Moving,      // moves left-right
}

#[derive(Clone)]
struct Brick {
    rect: Rect,
    color: Color,
    alive: bool,
    kind: BrickKind,
    hits: u32,
    max_hits: u32,
    move_dir: f32, // for Moving bricks
}

// ── Powerup types ──
#[derive(Clone, Copy, PartialEq)]
enum PowerupKind {
    MultiBall,
    WidePaddle,
    LaserUpgrade,
    SlowBall,
    ExtraLife,
    Shield,
    FireBall,    // ball pierces through bricks
    ShrinkPaddle, // negative
    FastBall,     // negative
    Magnet,       // ball sticks to paddle
}

const ALL_POWERUPS: [PowerupKind; 10] = [
    PowerupKind::MultiBall,
    PowerupKind::WidePaddle,
    PowerupKind::LaserUpgrade,
    PowerupKind::SlowBall,
    PowerupKind::ExtraLife,
    PowerupKind::Shield,
    PowerupKind::FireBall,
    PowerupKind::ShrinkPaddle,
    PowerupKind::FastBall,
    PowerupKind::Magnet,
];

struct Powerup {
    pos: Vec2,
    kind: PowerupKind,
    alive: bool,
}

impl PowerupKind {
    fn color(self) -> Color {
        match self {
            PowerupKind::MultiBall => SKYBLUE,
            PowerupKind::WidePaddle => GREEN,
            PowerupKind::LaserUpgrade => ORANGE,
            PowerupKind::SlowBall => BLUE,
            PowerupKind::ExtraLife => PINK,
            PowerupKind::Shield => Color::from_rgba(0, 255, 200, 255),
            PowerupKind::FireBall => Color::from_rgba(255, 80, 0, 255),
            PowerupKind::ShrinkPaddle => Color::from_rgba(180, 0, 0, 255),
            PowerupKind::FastBall => Color::from_rgba(180, 0, 0, 255),
            PowerupKind::Magnet => Color::from_rgba(200, 200, 0, 255),
        }
    }
    fn label(self) -> &'static str {
        match self {
            PowerupKind::MultiBall => "M",
            PowerupKind::WidePaddle => "W",
            PowerupKind::LaserUpgrade => "L",
            PowerupKind::SlowBall => "S",
            PowerupKind::ExtraLife => "+",
            PowerupKind::Shield => "=",
            PowerupKind::FireBall => "F",
            PowerupKind::ShrinkPaddle => "v",
            PowerupKind::FastBall => "!",
            PowerupKind::Magnet => "@",
        }
    }
    fn is_negative(self) -> bool {
        matches!(self, PowerupKind::ShrinkPaddle | PowerupKind::FastBall)
    }
}

#[derive(Clone)]
struct Ball {
    pos: Vec2,
    vel: Vec2,
    active: bool,
    fire: bool,
    stuck: bool, // magnet
    stuck_offset: f32,
}

struct Bullet {
    pos: Vec2,
    alive: bool,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    color: Color,
    life: f32,
}

struct FloatingText {
    pos: Vec2,
    text: String,
    color: Color,
    life: f32,
}

// ── Level patterns ──
fn level_bricks(level: u32) -> Vec<Brick> {
    let total_w = COLS as f32 * (BW + BGAP) - BGAP;
    let ox = (W - total_w) / 2.0;
    let mut bricks = Vec::new();

    let pattern = (level - 1) % 8;

    let rows: usize = match pattern {
        0 => 6,
        1 => 8,
        2 => 7,
        3 => 9,
        4 => 8,
        5 => 10,
        6 => 9,
        7 => 10,
        _ => 6,
    };

    for row in 0..rows {
        for col in 0..COLS {
            let x = ox + col as f32 * (BW + BGAP);
            let y = BTOP + row as f32 * (BH + BGAP);

            let present = match pattern {
                0 => true, // full grid
                1 => {
                    // diamond
                    let cr = (row as f32 - rows as f32 / 2.0).abs();
                    let cc = (col as f32 - COLS as f32 / 2.0 + 0.5).abs();
                    cr + cc < (rows.min(COLS) as f32 / 2.0 + 1.0)
                }
                2 => {
                    // checkerboard
                    (row + col) % 2 == 0
                }
                3 => {
                    // zigzag
                    let offset = if row % 2 == 0 { 0 } else { 1 };
                    (col + offset) % 3 != 0
                }
                4 => {
                    // V shape
                    let mid = COLS / 2;
                    let dist = if col < mid { mid - col } else { col - mid + 1 };
                    row < dist + 2
                }
                5 => {
                    // border + cross
                    row == 0 || row == rows - 1 || col == 0 || col == COLS - 1
                        || col == COLS / 2 || col == COLS / 2 - 1
                        || row == rows / 2
                }
                6 => {
                    // spiral-ish arcs
                    let r = row as f32;
                    let c = col as f32;
                    ((r * 1.5 + c) as u32) % 4 != 0
                }
                7 => {
                    // fortress
                    let is_wall = row < 2 || col == 0 || col == COLS - 1;
                    let is_tower = (col == 2 || col == COLS - 3) && row < 4;
                    is_wall || is_tower || (row == 4 && col > 2 && col < COLS - 3)
                }
                _ => true,
            };

            if !present {
                continue;
            }

            // Determine brick kind based on level difficulty
            let kind = determine_brick_kind(level, row, col, rows, pattern);
            let (hits, max_hits) = match kind {
                BrickKind::Normal => (1, 1),
                BrickKind::Tough => (2, 2),
                BrickKind::Armored => (3, 3),
                BrickKind::Explosive => (1, 1),
                BrickKind::Indestructible => (999, 999),
                BrickKind::Moving => (1, 1),
            };

            let color = brick_color(kind, row, level);

            bricks.push(Brick {
                rect: Rect::new(x, y, BW, BH),
                color,
                alive: true,
                kind,
                hits,
                max_hits,
                move_dir: if kind == BrickKind::Moving {
                    if col % 2 == 0 { 60.0 } else { -60.0 }
                } else {
                    0.0
                },
            });
        }
    }
    bricks
}

fn determine_brick_kind(level: u32, row: usize, col: usize, rows: usize, pattern: u32) -> BrickKind {
    // More special bricks at higher levels
    if pattern == 7 && (row < 2 || col == 0 || col == COLS - 1) && level >= 3 {
        return BrickKind::Armored;
    }
    if level >= 6 && row == 0 && col % 4 == 0 {
        return BrickKind::Indestructible;
    }
    if level >= 4 && row == rows / 2 && (col == COLS / 4 || col == 3 * COLS / 4) {
        return BrickKind::Explosive;
    }
    if level >= 5 && row % 3 == 0 && col % 5 == 2 {
        return BrickKind::Moving;
    }
    if level >= 3 && row < 2 {
        return BrickKind::Armored;
    }
    if row < (level.min(4) as usize) {
        return BrickKind::Tough;
    }
    if level >= 2 && (row + col) % 7 == 0 {
        return BrickKind::Explosive;
    }
    BrickKind::Normal
}

fn brick_color(kind: BrickKind, row: usize, level: u32) -> Color {
    match kind {
        BrickKind::Indestructible => Color::from_rgba(100, 100, 110, 255),
        BrickKind::Explosive => Color::from_rgba(255, 120, 30, 255),
        BrickKind::Moving => Color::from_rgba(255, 255, 100, 255),
        BrickKind::Armored => Color::from_rgba(180, 180, 200, 255),
        _ => {
            let hue = (row as f32 * 40.0 + level as f32 * 30.0) % 360.0;
            hsl_to_color(hue, 0.75, 0.55)
        }
    }
}

fn hsl_to_color(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::new(r + m, g + m, b + m, 1.0)
}

// ── Game state ──
#[derive(PartialEq)]
enum GameState {
    Menu,
    Playing,
    LevelClear,
    Lost,
}

struct Game {
    paddle_x: f32,
    paddle_w: f32,
    balls: Vec<Ball>,
    bricks: Vec<Brick>,
    bullets: Vec<Bullet>,
    powerups: Vec<Powerup>,
    particles: Vec<Particle>,
    floating_texts: Vec<FloatingText>,
    score: u32,
    lives: u32,
    level: u32,
    combo: u32,
    state: GameState,
    last_mouse_x: f32,
    ball_speed: f32,
    // Active effects
    laser_timer: f32,
    shield_active: bool,
    shield_hits: u32,
    fire_timer: f32,
    magnet_active: bool,
    magnet_timer: f32,
    shoot_cooldown: f32,
    // Timers for visual
    screen_shake: f32,
    level_clear_timer: f32,
    // Background stars
    stars: Vec<(f32, f32, f32)>,
}

impl Game {
    fn new() -> Self {
        let stars: Vec<(f32, f32, f32)> = (0..80)
            .map(|_| {
                (
                    rand::gen_range(0.0, W),
                    rand::gen_range(0.0, H),
                    rand::gen_range(0.3, 1.0),
                )
            })
            .collect();

        Game {
            paddle_x: (W - PADDLE_W_DEFAULT) / 2.0,
            paddle_w: PADDLE_W_DEFAULT,
            balls: Vec::new(),
            bricks: Vec::new(),
            bullets: Vec::new(),
            powerups: Vec::new(),
            particles: Vec::new(),
            floating_texts: Vec::new(),
            score: 0,
            lives: LIVES_INIT,
            level: 0,
            combo: 0,
            state: GameState::Menu,
            last_mouse_x: 0.0,
            ball_speed: BALL_SPEED_INIT,
            laser_timer: 0.0,
            shield_active: false,
            shield_hits: 0,
            fire_timer: 0.0,
            magnet_active: false,
            magnet_timer: 0.0,
            shoot_cooldown: 0.0,
            screen_shake: 0.0,
            level_clear_timer: 0.0,
            stars,
        }
    }

    fn start_level(&mut self, level: u32) {
        self.level = level;
        self.bricks = level_bricks(level);
        self.balls.clear();
        self.bullets.clear();
        self.powerups.clear();
        self.paddle_w = PADDLE_W_DEFAULT;
        self.ball_speed = BALL_SPEED_INIT + (level - 1) as f32 * 20.0;
        self.fire_timer = 0.0;
        self.magnet_active = false;
        self.magnet_timer = 0.0;
        self.combo = 0;
        self.spawn_ball_on_paddle();
        self.state = GameState::Playing;
    }

    fn spawn_ball_on_paddle(&mut self) {
        self.balls.push(Ball {
            pos: vec2(self.paddle_x + self.paddle_w / 2.0, PADDLE_Y - BALL_R - 2.0),
            vel: Vec2::ZERO,
            active: false,
            fire: false,
            stuck: false,
            stuck_offset: 0.0,
        });
    }

    fn launch_ball(ball: &mut Ball, speed: f32) {
        let angle = PI / 4.0 + rand::gen_range(0.0, PI / 2.0);
        ball.vel = vec2(angle.cos(), -angle.sin()) * speed;
        ball.active = true;
        ball.stuck = false;
    }

    fn spawn_particles(&mut self, pos: Vec2, color: Color, count: usize) {
        for _ in 0..count {
            let angle = rand::gen_range(0.0, PI * 2.0);
            let speed = rand::gen_range(60.0, 200.0);
            self.particles.push(Particle {
                pos,
                vel: vec2(angle.cos() * speed, angle.sin() * speed),
                color,
                life: rand::gen_range(0.3, 0.7),
            });
        }
    }

    fn spawn_floating_text(&mut self, pos: Vec2, text: String, color: Color) {
        self.floating_texts.push(FloatingText {
            pos,
            text,
            color,
            life: 1.0,
        });
    }

    fn maybe_drop_powerup(&mut self, pos: Vec2) {
        if rand::gen_range(0.0, 1.0) < PU_DROP_CHANCE {
            // Negative powerups become more likely at higher levels
            let negative_chance = (self.level as f32 * 0.04).min(0.35);
            let kind = if rand::gen_range(0.0, 1.0) < negative_chance {
                if rand::gen_range(0.0, 1.0) < 0.5 {
                    PowerupKind::ShrinkPaddle
                } else {
                    PowerupKind::FastBall
                }
            } else {
                let positives: Vec<PowerupKind> = ALL_POWERUPS
                    .iter()
                    .copied()
                    .filter(|p| !p.is_negative())
                    .collect();
                positives[rand::gen_range(0, positives.len())]
            };
            self.powerups.push(Powerup {
                pos,
                kind,
                alive: true,
            });
        }
    }

    fn apply_powerup(&mut self, kind: PowerupKind) {
        match kind {
            PowerupKind::MultiBall => {
                let existing: Vec<Ball> = self.balls.iter().filter(|b| b.active).cloned().collect();
                for b in existing.iter().take(3) {
                    for angle_offset in &[-0.4, 0.4] {
                        let speed = b.vel.length();
                        let angle = b.vel.y.atan2(b.vel.x) + angle_offset;
                        self.balls.push(Ball {
                            pos: b.pos,
                            vel: vec2(angle.cos(), angle.sin()) * speed,
                            active: true,
                            fire: b.fire,
                            stuck: false,
                            stuck_offset: 0.0,
                        });
                    }
                }
            }
            PowerupKind::WidePaddle => {
                self.paddle_w = (self.paddle_w + 40.0).min(240.0);
            }
            PowerupKind::ShrinkPaddle => {
                self.paddle_w = (self.paddle_w - 30.0).max(50.0);
                self.screen_shake = 0.15;
            }
            PowerupKind::LaserUpgrade => {
                self.laser_timer = 15.0; // extra rapid fire period
            }
            PowerupKind::SlowBall => {
                for ball in &mut self.balls {
                    let speed = ball.vel.length();
                    if speed > 200.0 {
                        let dir = ball.vel.normalize();
                        ball.vel = dir * (speed * 0.6);
                    }
                }
                self.ball_speed = (self.ball_speed * 0.7).max(250.0);
            }
            PowerupKind::FastBall => {
                for ball in &mut self.balls {
                    let speed = ball.vel.length();
                    let dir = ball.vel.normalize();
                    ball.vel = dir * (speed * 1.4).min(BALL_SPEED_MAX);
                }
                self.ball_speed = (self.ball_speed * 1.3).min(BALL_SPEED_MAX);
                self.screen_shake = 0.2;
            }
            PowerupKind::ExtraLife => {
                self.lives += 1;
            }
            PowerupKind::Shield => {
                self.shield_active = true;
                self.shield_hits = 3;
            }
            PowerupKind::FireBall => {
                self.fire_timer = 10.0;
                for ball in &mut self.balls {
                    ball.fire = true;
                }
            }
            PowerupKind::Magnet => {
                self.magnet_active = true;
                self.magnet_timer = 12.0;
            }
        }
    }

    fn explode_around(&mut self, center: Rect) {
        let cx = center.x + center.w / 2.0;
        let cy = center.y + center.h / 2.0;
        let radius = BW * 2.0;
        self.screen_shake = 0.3;
        self.spawn_particles(vec2(cx, cy), ORANGE, 20);

        let mut to_explode = Vec::new();
        let mut deferred_particles: Vec<(Vec2, Color)> = Vec::new();
        for (i, brick) in self.bricks.iter_mut().enumerate() {
            if !brick.alive || brick.kind == BrickKind::Indestructible {
                continue;
            }
            let bx = brick.rect.x + brick.rect.w / 2.0;
            let by = brick.rect.y + brick.rect.h / 2.0;
            let dist = ((bx - cx).powi(2) + (by - cy).powi(2)).sqrt();
            if dist < radius {
                brick.hits = 0;
                brick.alive = false;
                self.score += 10;
                deferred_particles.push((vec2(bx, by), brick.color));
                if brick.kind == BrickKind::Explosive {
                    to_explode.push(i);
                }
            }
        }
        for (pos, color) in deferred_particles {
            self.spawn_particles(pos, color, 4);
        }
        // Chain explosions
        for idx in to_explode {
            let rect = self.bricks[idx].rect;
            self.explode_around(rect);
        }
    }

    fn update(&mut self) {
        let dt = get_frame_time().min(0.033);

        // Timers
        if self.screen_shake > 0.0 {
            self.screen_shake -= dt;
        }
        if self.laser_timer > 0.0 {
            self.laser_timer -= dt;
        }
        if self.fire_timer > 0.0 {
            self.fire_timer -= dt;
            if self.fire_timer <= 0.0 {
                for ball in &mut self.balls {
                    ball.fire = false;
                }
            }
        }
        if self.magnet_timer > 0.0 {
            self.magnet_timer -= dt;
            if self.magnet_timer <= 0.0 {
                self.magnet_active = false;
            }
        }
        if self.shoot_cooldown > 0.0 {
            self.shoot_cooldown -= dt;
        }

        // ── Paddle movement ──
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.paddle_x -= PADDLE_SPEED * dt;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.paddle_x += PADDLE_SPEED * dt;
        }
        let (mx, _) = mouse_position();
        if (mx - self.last_mouse_x).abs() > 0.5 {
            self.paddle_x = mx - self.paddle_w / 2.0;
        }
        self.last_mouse_x = mx;
        self.paddle_x = self.paddle_x.clamp(0.0, W - self.paddle_w);

        // ── Shooting (always available) ──
        let rapid = self.laser_timer > 0.0;
        let cooldown = if rapid { SHOOT_COOLDOWN * 0.4 } else { SHOOT_COOLDOWN };
        if (is_mouse_button_down(MouseButton::Left) || is_key_down(KeyCode::Space))
            && self.shoot_cooldown <= 0.0
            && self.balls.iter().any(|b| b.active)
        {
            self.shoot_cooldown = cooldown;
            let px = self.paddle_x;
            let pw = self.paddle_w;
            self.bullets.push(Bullet {
                pos: vec2(px + 4.0, PADDLE_Y - BULLET_H),
                alive: true,
            });
            self.bullets.push(Bullet {
                pos: vec2(px + pw - 4.0 - BULLET_W, PADDLE_Y - BULLET_H),
                alive: true,
            });
        }

        // ── Balls on paddle / magnet ──
        let any_active = self.balls.iter().any(|b| b.active);
        for ball in &mut self.balls {
            if !ball.active && !ball.stuck {
                ball.pos.x = self.paddle_x + self.paddle_w / 2.0;
                ball.pos.y = PADDLE_Y - BALL_R - 2.0;
            }
            if ball.stuck {
                ball.pos.x = self.paddle_x + ball.stuck_offset;
                ball.pos.y = PADDLE_Y - BALL_R - 2.0;
            }
        }

        if !any_active || self.balls.iter().any(|b| b.stuck) {
            if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
                let speed = self.ball_speed;
                for ball in &mut self.balls {
                    if !ball.active || ball.stuck {
                        Self::launch_ball(ball, speed);
                    }
                }
            }
        }

        // ── Move bricks (Moving type) ──
        for brick in &mut self.bricks {
            if brick.alive && brick.kind == BrickKind::Moving {
                brick.rect.x += brick.move_dir * dt;
                if brick.rect.x <= 0.0 || brick.rect.x + brick.rect.w >= W {
                    brick.move_dir = -brick.move_dir;
                }
            }
        }

        // ── Update particles ──
        for p in &mut self.particles {
            p.pos += p.vel * dt;
            p.vel *= 0.96;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        // ── Update floating texts ──
        for ft in &mut self.floating_texts {
            ft.pos.y -= 40.0 * dt;
            ft.life -= dt;
        }
        self.floating_texts.retain(|ft| ft.life > 0.0);

        // ── Update bullets ──
        // Move bullets
        for bullet in &mut self.bullets {
            if bullet.alive {
                bullet.pos.y -= BULLET_SPEED * dt;
                if bullet.pos.y < 0.0 {
                    bullet.alive = false;
                }
            }
        }
        // Check bullet-brick collisions (separate pass to avoid borrow issues)
        let mut bullet_kills = Vec::new();
        let mut bullet_particles: Vec<(Vec2, Color)> = Vec::new();
        let mut bullet_powerup_drops: Vec<Vec2> = Vec::new();
        for bullet in &mut self.bullets {
            if !bullet.alive { continue; }
            let brect = Rect::new(bullet.pos.x, bullet.pos.y, BULLET_W, BULLET_H);
            for (i, brick) in self.bricks.iter_mut().enumerate() {
                if !brick.alive || brick.kind == BrickKind::Indestructible { continue; }
                if brect.overlaps(&brick.rect) {
                    bullet.alive = false;
                    brick.hits = brick.hits.saturating_sub(1);
                    if brick.hits == 0 {
                        brick.alive = false;
                        self.score += 5;
                        let center = vec2(brick.rect.x + BW / 2.0, brick.rect.y + BH / 2.0);
                        bullet_particles.push((center, brick.color));
                        bullet_powerup_drops.push(center);
                        if brick.kind == BrickKind::Explosive {
                            bullet_kills.push(i);
                        }
                    }
                    break;
                }
            }
        }
        for (pos, color) in bullet_particles {
            self.spawn_particles(pos, color, 4);
        }
        for pos in bullet_powerup_drops {
            self.maybe_drop_powerup(pos);
        }
        for idx in bullet_kills {
            let rect = self.bricks[idx].rect;
            self.explode_around(rect);
        }
        self.bullets.retain(|b| b.alive);

        // ── Update powerups ──
        let paddle_rect = Rect::new(self.paddle_x, PADDLE_Y, self.paddle_w, PADDLE_H);
        let mut collected = Vec::new();
        for pu in &mut self.powerups {
            if !pu.alive {
                continue;
            }
            pu.pos.y += PU_SPEED * dt;
            if pu.pos.y > H {
                pu.alive = false;
                continue;
            }
            let pr = Rect::new(pu.pos.x - PU_SIZE / 2.0, pu.pos.y - PU_SIZE / 2.0, PU_SIZE, PU_SIZE);
            if pr.overlaps(&paddle_rect) {
                pu.alive = false;
                collected.push(pu.kind);
            }
        }
        self.powerups.retain(|p| p.alive);
        for kind in collected {
            let label = if kind.is_negative() {
                format!("{}", match kind {
                    PowerupKind::ShrinkPaddle => "Shrink!",
                    PowerupKind::FastBall => "Speed Up!",
                    _ => "!",
                })
            } else {
                format!("{}", match kind {
                    PowerupKind::MultiBall => "Multi Ball!",
                    PowerupKind::WidePaddle => "Wide Paddle!",
                    PowerupKind::LaserUpgrade => "Rapid Fire!",
                    PowerupKind::SlowBall => "Slow Ball!",
                    PowerupKind::ExtraLife => "Extra Life!",
                    PowerupKind::Shield => "Shield!",
                    PowerupKind::FireBall => "Fire Ball!",
                    PowerupKind::Magnet => "Magnet!",
                    _ => "!",
                })
            };
            let color = if kind.is_negative() { RED } else { GOLD };
            self.spawn_floating_text(vec2(self.paddle_x + self.paddle_w / 2.0, PADDLE_Y - 30.0), label, color);
            self.apply_powerup(kind);
        }

        // ── Update balls ──
        let paddle_rect = Rect::new(self.paddle_x, PADDLE_Y, self.paddle_w, PADDLE_H);
        let ball_speed = self.ball_speed;
        let magnet = self.magnet_active;

        let mut explosions = Vec::new();
        let mut new_score: u32 = 0;
        let mut combo = self.combo;

        for ball in &mut self.balls {
            if !ball.active || ball.stuck {
                continue;
            }

            ball.pos += ball.vel * dt;

            // Wall collisions
            if ball.pos.x - BALL_R <= 0.0 {
                ball.pos.x = BALL_R;
                ball.vel.x = ball.vel.x.abs();
            }
            if ball.pos.x + BALL_R >= W {
                ball.pos.x = W - BALL_R;
                ball.vel.x = -ball.vel.x.abs();
            }
            if ball.pos.y - BALL_R <= 0.0 {
                ball.pos.y = BALL_R;
                ball.vel.y = ball.vel.y.abs();
            }

            // Fall below
            if ball.pos.y > H + BALL_R {
                if self.shield_active {
                    ball.pos.y = H - BALL_R;
                    ball.vel.y = -ball.vel.y.abs();
                    self.shield_hits = self.shield_hits.saturating_sub(1);
                    if self.shield_hits == 0 {
                        self.shield_active = false;
                    }
                } else {
                    ball.active = false;
                }
                continue;
            }

            // Paddle collision
            if ball.vel.y > 0.0 && ball_rect_collision(ball.pos, BALL_R, &paddle_rect) {
                ball.pos.y = PADDLE_Y - BALL_R;
                if magnet {
                    ball.stuck = true;
                    ball.stuck_offset = ball.pos.x - self.paddle_x;
                    ball.vel = Vec2::ZERO;
                } else {
                    let hit_pos = (ball.pos.x - self.paddle_x) / self.paddle_w;
                    let angle = PI * (0.15 + 0.7 * (1.0 - hit_pos));
                    let speed = ball.vel.length().max(ball_speed);
                    ball.vel = vec2(angle.cos(), -angle.sin()) * speed;
                }
                combo = 0;
            }

            // Brick collisions
            for (i, brick) in self.bricks.iter_mut().enumerate() {
                if !brick.alive {
                    continue;
                }
                if ball_rect_collision(ball.pos, BALL_R, &brick.rect) {
                    if brick.kind == BrickKind::Indestructible {
                        // Just bounce
                        reflect_ball(ball, &brick.rect);
                        break;
                    }

                    brick.hits = brick.hits.saturating_sub(1);
                    let destroyed = brick.hits == 0;

                    if destroyed {
                        brick.alive = false;
                        combo += 1;
                        let base = match brick.kind {
                            BrickKind::Armored => 30,
                            BrickKind::Tough => 20,
                            BrickKind::Explosive => 25,
                            BrickKind::Moving => 25,
                            _ => 10,
                        };
                        new_score += base * combo;

                        if brick.kind == BrickKind::Explosive {
                            explosions.push(i);
                        }
                    }

                    // Fire ball pierces, others bounce
                    if !ball.fire {
                        reflect_ball(ball, &brick.rect);
                    }

                    // Speed up
                    let speed = ball.vel.length();
                    let new_speed = (speed + BALL_SPEED_INC).min(BALL_SPEED_MAX);
                    if speed > 0.0 {
                        ball.vel = ball.vel.normalize() * new_speed;
                    }

                    if !ball.fire {
                        break;
                    }
                }
            }
        }

        self.combo = combo;
        self.score += new_score;

        // Handle explosions
        for idx in explosions {
            let rect = self.bricks[idx].rect;
            self.explode_around(rect);
        }

        // Spawn particles for destroyed bricks (not yet handled)
        // Clean up dead bricks - spawn particles + powerups
        // (We do this by checking what just died - the alive field was set to false above)

        // Remove dead balls
        let had_active = self.balls.iter().any(|b| b.active || b.stuck);
        self.balls.retain(|b| b.active || b.stuck || !had_active);

        // Lost a ball?
        if had_active && !self.balls.iter().any(|b| b.active || b.stuck) {
            self.lives = self.lives.saturating_sub(1);
            self.combo = 0;
            if self.lives == 0 {
                self.state = GameState::Lost;
            } else {
                self.paddle_w = PADDLE_W_DEFAULT;
                self.spawn_ball_on_paddle();
            }
        }

        // Check level clear (all destroyable bricks gone)
        let destroyable_left = self
            .bricks
            .iter()
            .any(|b| b.alive && b.kind != BrickKind::Indestructible);
        if !destroyable_left && self.state == GameState::Playing {
            self.state = GameState::LevelClear;
            self.level_clear_timer = 2.0;
            self.score += 500 * self.level; // level bonus
        }
    }

    fn draw(&self) {
        let shake = if self.screen_shake > 0.0 {
            vec2(
                rand::gen_range(-3.0, 3.0),
                rand::gen_range(-3.0, 3.0),
            )
        } else {
            Vec2::ZERO
        };

        clear_background(Color::from_rgba(10, 10, 25, 255));

        // Background stars
        let t = get_time() as f32;
        for &(sx, sy, bright) in &self.stars {
            let flicker = (t * bright * 3.0).sin() * 0.3 + 0.7;
            let a = (bright * flicker * 255.0) as u8;
            draw_circle(sx + shake.x, sy + shake.y, 1.0 + bright * 0.5, Color::from_rgba(200, 200, 255, a));
        }

        // Shield bar at bottom
        if self.shield_active {
            let alpha = ((t * 4.0).sin() * 30.0 + 200.0) as u8;
            draw_rectangle(0.0, H - 6.0 + shake.y, W, 6.0, Color::from_rgba(0, 255, 200, alpha));
        }

        // ── Bricks ──
        for brick in &self.bricks {
            if !brick.alive {
                continue;
            }
            let bx = brick.rect.x + shake.x;
            let by = brick.rect.y + shake.y;

            match brick.kind {
                BrickKind::Indestructible => {
                    draw_rectangle(bx, by, BW, BH, Color::from_rgba(80, 80, 95, 255));
                    draw_rectangle_lines(bx, by, BW, BH, 2.0, Color::from_rgba(130, 130, 150, 255));
                    // X pattern
                    draw_line(bx + 3.0, by + 3.0, bx + BW - 3.0, by + BH - 3.0, 1.5,
                        Color::from_rgba(130, 130, 150, 100));
                    draw_line(bx + BW - 3.0, by + 3.0, bx + 3.0, by + BH - 3.0, 1.5,
                        Color::from_rgba(130, 130, 150, 100));
                }
                BrickKind::Explosive => {
                    let pulse = ((t * 5.0).sin() * 0.15 + 0.85) as f32;
                    let c = Color::new(1.0, 0.4 * pulse, 0.1, 1.0);
                    draw_rectangle(bx, by, BW, BH, c);
                    draw_rectangle_lines(bx, by, BW, BH, 2.0, ORANGE);
                    // Bomb symbol
                    let cx = bx + BW / 2.0;
                    let cy = by + BH / 2.0;
                    draw_circle(cx, cy, 5.0, Color::from_rgba(50, 50, 50, 200));
                    draw_line(cx, cy - 5.0, cx + 3.0, cy - 9.0, 2.0, ORANGE);
                }
                BrickKind::Moving => {
                    let hue = (t * 100.0 + brick.rect.x) % 360.0;
                    let c = hsl_to_color(hue, 0.9, 0.6);
                    draw_rectangle(bx, by, BW, BH, c);
                    draw_rectangle_lines(bx, by, BW, BH, 2.0, WHITE);
                    // Arrow indicators
                    if brick.move_dir > 0.0 {
                        draw_text(">", bx + BW - 14.0, by + BH - 5.0, 18.0, WHITE);
                    } else {
                        draw_text("<", bx + 3.0, by + BH - 5.0, 18.0, WHITE);
                    }
                }
                _ => {
                    draw_rectangle(bx, by, BW, BH, brick.color);
                    // Highlight top edge
                    let lighter = Color::new(
                        (brick.color.r + 0.2).min(1.0),
                        (brick.color.g + 0.2).min(1.0),
                        (brick.color.b + 0.2).min(1.0),
                        0.6,
                    );
                    draw_rectangle(bx, by, BW, 3.0, lighter);
                    draw_rectangle_lines(bx, by, BW, BH, 1.0, Color::from_rgba(0, 0, 0, 60));

                    // Damage cracks
                    if brick.max_hits >= 2 && brick.hits < brick.max_hits {
                        let cx = bx + BW / 2.0;
                        let cy = by + BH / 2.0;
                        let damage = brick.max_hits - brick.hits;
                        draw_line(cx - 8.0, cy - 4.0, cx + 4.0, cy + 2.0, 1.5,
                            Color::from_rgba(0, 0, 0, 140));
                        if damage >= 2 {
                            draw_line(cx + 2.0, cy - 5.0, cx - 6.0, cy + 4.0, 1.5,
                                Color::from_rgba(0, 0, 0, 140));
                        }
                    }

                    // HP indicator for armored
                    if brick.kind == BrickKind::Armored && brick.hits > 1 {
                        let label = format!("{}", brick.hits);
                        let tw = measure_text(&label, None, 14, 1.0).width;
                        draw_text(&label, bx + BW / 2.0 - tw / 2.0, by + BH - 5.0, 14.0,
                            Color::from_rgba(255, 255, 255, 180));
                    }
                }
            }
        }

        // ── Bullets ──
        for bullet in &self.bullets {
            if !bullet.alive {
                continue;
            }
            let bx = bullet.pos.x + shake.x;
            let by = bullet.pos.y + shake.y;
            // Glow
            draw_rectangle(bx - 1.0, by, BULLET_W + 2.0, BULLET_H, Color::from_rgba(255, 200, 50, 60));
            draw_rectangle(bx, by, BULLET_W, BULLET_H, Color::from_rgba(255, 220, 80, 255));
        }

        // ── Powerups ──
        for pu in &self.powerups {
            if !pu.alive {
                continue;
            }
            let px = pu.pos.x + shake.x;
            let py = pu.pos.y + shake.y;
            let c = pu.kind.color();
            // Glow
            draw_circle(px, py, PU_SIZE * 0.7, Color::new(c.r, c.g, c.b, 0.25));
            // Diamond shape
            let s = PU_SIZE / 2.0;
            let rot = t * 2.0;
            draw_poly(px, py, 4, s, rot.to_degrees(), c);
            draw_poly_lines(px, py, 4, s + 1.0, rot.to_degrees(), 1.5, WHITE);
            // Label
            let label = pu.kind.label();
            let tw = measure_text(label, None, 14, 1.0).width;
            draw_text(label, px - tw / 2.0, py + 5.0, 14.0, WHITE);
        }

        // ── Particles ──
        for p in &self.particles {
            let alpha = (p.life * 255.0).min(255.0) as u8;
            let c = Color::from_rgba(
                (p.color.r * 255.0) as u8,
                (p.color.g * 255.0) as u8,
                (p.color.b * 255.0) as u8,
                alpha,
            );
            let size = p.life * 4.0;
            draw_circle(p.pos.x + shake.x, p.pos.y + shake.y, size, c);
        }

        // ── Paddle ──
        let px = self.paddle_x + shake.x;
        let py = PADDLE_Y + shake.y;
        // Glow under paddle
        draw_rectangle(px - 2.0, py + PADDLE_H, self.paddle_w + 4.0, 4.0,
            Color::from_rgba(100, 150, 255, 40));
        draw_rectangle(px, py, self.paddle_w, PADDLE_H, WHITE);
        draw_rectangle(px + 2.0, py + 2.0, self.paddle_w - 4.0, PADDLE_H - 6.0,
            Color::from_rgba(180, 200, 255, 255));
        // Laser indicators (always on)
        let gun_color = if self.laser_timer > 0.0 {
            Color::from_rgba(255, 150, 50, 255)
        } else {
            Color::from_rgba(255, 220, 100, 200)
        };
        draw_rectangle(px + 2.0, py - 4.0, 4.0, 4.0, gun_color);
        draw_rectangle(px + self.paddle_w - 6.0, py - 4.0, 4.0, 4.0, gun_color);

        // ── Balls ──
        for ball in &self.balls {
            if !ball.active && !ball.stuck {
                if self.balls.iter().any(|b| b.active) {
                    continue;
                }
            }
            let bx = ball.pos.x + shake.x;
            let by = ball.pos.y + shake.y;
            if ball.fire {
                // Fire trail
                for i in 1..4 {
                    let offset = i as f32 * 3.0;
                    let alpha = (180 - i * 50).max(0) as u8;
                    draw_circle(
                        bx - ball.vel.x.signum() * offset,
                        by - ball.vel.y.signum() * offset,
                        BALL_R * 0.7,
                        Color::from_rgba(255, 100, 0, alpha),
                    );
                }
                draw_circle(bx, by, BALL_R + 1.0, Color::from_rgba(255, 80, 0, 180));
                draw_circle(bx, by, BALL_R, Color::from_rgba(255, 200, 50, 255));
            } else {
                draw_circle(bx, by, BALL_R + 1.0, Color::from_rgba(100, 150, 255, 80));
                draw_circle(bx, by, BALL_R, WHITE);
                draw_circle(bx - 2.0, by - 2.0, BALL_R * 0.35, Color::from_rgba(255, 255, 255, 120));
            }
        }

        // ── Floating texts ──
        for ft in &self.floating_texts {
            let alpha = (ft.life * 255.0).min(255.0) as u8;
            let c = Color::from_rgba(
                (ft.color.r * 255.0) as u8,
                (ft.color.g * 255.0) as u8,
                (ft.color.b * 255.0) as u8,
                alpha,
            );
            let tw = measure_text(&ft.text, None, 20, 1.0).width;
            draw_text(&ft.text, ft.pos.x - tw / 2.0 + shake.x, ft.pos.y + shake.y, 20.0, c);
        }

        // ── HUD ──
        draw_text(&format!("Score: {}", self.score), 10.0, 28.0, 28.0, WHITE);
        draw_text(&format!("Level {}", self.level), W / 2.0 - 30.0, 28.0, 28.0,
            Color::from_rgba(200, 200, 255, 200));

        // Lives
        for i in 0..self.lives {
            draw_circle(W - 25.0 - i as f32 * 22.0, 20.0, 7.0, RED);
        }

        // Combo
        if self.combo > 1 {
            let txt = format!("COMBO x{}!", self.combo);
            let tw = measure_text(&txt, None, 26, 1.0).width;
            let pulse = ((t * 6.0).sin() * 20.0 + 235.0) as u8;
            draw_text(&txt, W / 2.0 - tw / 2.0, 52.0, 26.0, Color::from_rgba(255, pulse, 50, 255));
        }

        // Active effects indicator
        let mut ey = 70.0;
        if self.fire_timer > 0.0 {
            draw_text(&format!("FIRE {:.0}s", self.fire_timer), 10.0, ey, 18.0,
                Color::from_rgba(255, 100, 0, 220));
            ey += 18.0;
        }
        if self.laser_timer > 0.0 {
            draw_text(&format!("RAPID {:.0}s", self.laser_timer), 10.0, ey, 18.0, ORANGE);
            ey += 18.0;
        }
        if self.magnet_active {
            draw_text(&format!("MAGNET {:.0}s", self.magnet_timer), 10.0, ey, 18.0, YELLOW);
            ey += 18.0;
        }
        if self.shield_active {
            draw_text(&format!("SHIELD x{}", self.shield_hits), 10.0, ey, 18.0,
                Color::from_rgba(0, 255, 200, 220));
            let _ = ey;
        }

        // Launch hint
        if !self.balls.iter().any(|b| b.active) {
            let hint = "Click to launch";
            let tw = measure_text(hint, None, 22, 1.0).width;
            let alpha = ((t * 3.0).sin() * 60.0 + 180.0) as u8;
            draw_text(hint, W / 2.0 - tw / 2.0, PADDLE_Y - 40.0, 22.0,
                Color::from_rgba(255, 255, 255, alpha));
        }
    }
}

fn ball_rect_collision(bp: Vec2, r: f32, rect: &Rect) -> bool {
    let cx = bp.x.clamp(rect.x, rect.x + rect.w);
    let cy = bp.y.clamp(rect.y, rect.y + rect.h);
    let d = vec2(bp.x - cx, bp.y - cy);
    d.length_squared() <= r * r
}

fn reflect_ball(ball: &mut Ball, rect: &Rect) {
    let bx = ball.pos.x.clamp(rect.x, rect.x + rect.w);
    let by = ball.pos.y.clamp(rect.y, rect.y + rect.h);
    let dx = ball.pos.x - bx;
    let dy = ball.pos.y - by;
    if dx.abs() > dy.abs() {
        ball.vel.x = -ball.vel.x;
    } else {
        ball.vel.y = -ball.vel.y;
    }
}

fn draw_centered(text: &str, y: f32, size: f32, color: Color) {
    let tw = measure_text(text, None, size as u16, 1.0).width;
    draw_text(text, W / 2.0 - tw / 2.0, y, size, color);
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Breakout Deluxe".to_owned(),
        window_width: W as i32,
        window_height: H as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();

    loop {
        match game.state {
            GameState::Menu => {
                clear_background(Color::from_rgba(10, 10, 25, 255));
                let t = get_time() as f32;

                // Animated title
                let title = "BREAKOUT DELUXE";
                for (i, ch) in title.chars().enumerate() {
                    let hue = (i as f32 * 30.0 + t * 60.0) % 360.0;
                    let c = hsl_to_color(hue, 0.9, 0.6);
                    let x = W / 2.0 - 180.0 + i as f32 * 26.0;
                    let y = H / 2.0 - 80.0 + (t * 2.0 + i as f32 * 0.3).sin() * 5.0;
                    draw_text(&ch.to_string(), x, y, 48.0, c);
                }

                draw_centered("Click to start", H / 2.0 - 10.0, 28.0, WHITE);
                draw_centered("Mouse to move | Click to shoot | SPACE to launch ball", H / 2.0 + 30.0, 18.0, GRAY);

                // Feature list
                draw_centered("10 Powerups | Explosive Bricks | Moving Bricks | Fire Ball", H / 2.0 + 60.0, 16.0,
                    Color::from_rgba(150, 150, 200, 200));
                draw_centered("8 Unique Levels | Combo System | Built-in Lasers", H / 2.0 + 82.0, 16.0,
                    Color::from_rgba(150, 150, 200, 200));

                // Background stars
                for &(sx, sy, bright) in &game.stars {
                    let a = (bright * ((t * bright * 3.0).sin() * 0.3 + 0.7) * 255.0) as u8;
                    draw_circle(sx, sy, 1.0, Color::from_rgba(200, 200, 255, a));
                }

                if is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left) {
                    game.start_level(1);
                }
            }

            GameState::Playing => {
                game.update();
                game.draw();
            }

            GameState::LevelClear => {
                game.draw();
                let dt = get_frame_time();
                game.level_clear_timer -= dt;

                draw_rectangle(0.0, H / 2.0 - 70.0, W, 140.0, Color::from_rgba(0, 0, 0, 200));
                let t = get_time() as f32;
                let hue = (t * 60.0) % 360.0;
                let c = hsl_to_color(hue, 0.9, 0.65);
                draw_centered(&format!("LEVEL {} CLEAR!", game.level), H / 2.0 - 25.0, 48.0, c);
                draw_centered(&format!("Bonus: +{}", 500 * game.level), H / 2.0 + 15.0, 28.0, GOLD);
                draw_centered("Click for next level", H / 2.0 + 50.0, 22.0, GRAY);

                if game.level_clear_timer <= 0.0
                    && (is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left))
                {
                    game.start_level(game.level + 1);
                }
            }

            GameState::Lost => {
                game.draw();
                draw_rectangle(0.0, H / 2.0 - 70.0, W, 140.0, Color::from_rgba(0, 0, 0, 200));
                draw_centered("GAME OVER", H / 2.0 - 25.0, 52.0, RED);
                draw_centered(&format!("Final Score: {}  |  Level: {}", game.score, game.level),
                    H / 2.0 + 15.0, 28.0, WHITE);
                draw_centered("Click to retry", H / 2.0 + 50.0, 22.0, GRAY);

                if is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left) {
                    game = Game::new();
                    game.start_level(1);
                }
            }
        }

        next_frame().await;
    }
}

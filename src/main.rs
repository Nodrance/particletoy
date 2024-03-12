// fuck the needless return lint
#![allow(clippy::needless_return)]

use std::vec;

use macroquad::prelude::*;
use macroquad::rand::ChooseRandom;

const PIXEL_SCALE: f32 = 2.5;

#[derive(Clone, Copy, Debug)]
struct FluidParticle {
    position: Vec2,
    velocity: Vec2,
    acceleration: Vec2,
    mass: f32,
    radius: f32,
    color: Color,
}

//Takes in a distance and gives a repelling force quantity
fn getinfluence(distance_fraction: f32) -> f32 {
    if distance_fraction > 2.0 {
        return 0.0;
    }
    let influence = if distance_fraction < 0.5 {
        1.0 - (6.0 * distance_fraction.powi(2)) + (6.0 * distance_fraction.powi(3))
    } else {
        2.0 * (1.0 - distance_fraction).powi(3)
    };
    return influence;
}

impl FluidParticle {
    fn new(position: Option<Vec2>, mass: f32, radius: f32, color: Color) -> Self {
        let position = position.unwrap_or(Vec2::new(0.0, 0.0));
        Self {
            position,
            velocity: Vec2::new(0.0, 0.0),
            acceleration: Vec2::new(0.0, 0.0),
            mass,
            radius,
            color,
        }
    }

    fn apply_force(&mut self, force: Vec2) {
        self.acceleration += force / self.mass;
    }

    fn update(&mut self) {
        self.velocity += self.acceleration;
        self.position += self.velocity/8.0;
        self.acceleration *= 0.0;
        const GENERAL_DAMPING: f32 = 0.99;
        self.velocity *= GENERAL_DAMPING;
    }

    fn draw(&self) {
        draw_circle(self.position.x * PIXEL_SCALE, self.position.y * PIXEL_SCALE, self.radius * PIXEL_SCALE, self.color);
    }
}
impl Default for FluidParticle {
    fn default() -> Self {
        Self::new(None, 1.0, 5.0, WHITE)
    }
}
#[derive(Clone, Debug)]
struct FluidBox {
    particles: Vec<FluidParticle>,
    x_pos: f32,
    y_pos: f32,
    width: f32,
    height: f32,
    color: Color,
    render_target: RenderTarget,
}
impl FluidBox {
    fn new(x_pos: f32, y_pos: f32, width: f32, height: f32, color: Color) -> Self {
        Self {
            x_pos,
            y_pos,
            particles: vec![],
            width,
            height,
            color,
            render_target: render_target((width * PIXEL_SCALE) as u32, (height * PIXEL_SCALE) as u32),
        }
    }

    fn add_particle(&mut self, particle: FluidParticle) {
        self.particles.push(particle);
    }

    fn apply_force(&mut self, force: Vec2) {
        for particle in self.particles.iter_mut() {
            particle.apply_force(force);
        }
    }

    fn update(&mut self, particle_state: ParticleType) {
        let particle_length = self.particles.len();
        for i in 0..particle_length {
            let particle = &mut self.particles[i].clone();
            const BOUNCE_DAMPING: f32 = 0.95;
            const WALL_DISTANCE: f32 = 20.0;
            const WALL_FORCE: f32 = 0.5;
            { // Walls
                if particle.position.x < WALL_DISTANCE {
                    if particle.position.x < particle.radius {
                        particle.position.x = particle.radius;
                        particle.velocity.x = 0.0;
                    }
                    particle.velocity.x *= BOUNCE_DAMPING;
                    particle.apply_force(Vec2::new(WALL_FORCE*getinfluence(particle.position.x/WALL_DISTANCE), 0.0));
                }
                if particle.position.x > self.width - WALL_DISTANCE {
                    if particle.position.x > self.width - particle.radius {
                        particle.position.x = self.width - particle.radius;
                        particle.velocity.x = 0.0;
                    }
                    particle.velocity.x *= BOUNCE_DAMPING;
                    particle.apply_force(Vec2::new(-WALL_FORCE*getinfluence((self.width - particle.position.x)/WALL_DISTANCE), 0.0));
                }
                if particle.position.y < WALL_DISTANCE {
                    if particle.position.y < particle.radius {
                        particle.position.y = particle.radius;
                        particle.velocity.y = 0.0;
                    }
                    particle.velocity.y *= BOUNCE_DAMPING;
                    particle.apply_force(Vec2::new(0.0, WALL_FORCE*getinfluence(particle.position.y/WALL_DISTANCE)));
                }
                if particle.position.y > self.height - WALL_DISTANCE {
                    if particle.position.y > self.height - particle.radius {
                        particle.position.y = self.height - particle.radius;
                        particle.velocity.y = 0.0;
                    }
                    particle.velocity.y *= BOUNCE_DAMPING;
                    particle.apply_force(Vec2::new(0.0, -WALL_FORCE*getinfluence((self.height - particle.position.y)/WALL_DISTANCE)));
                }
            }
            for j in i+1..particle_length {
                let particle2 = &mut self.particles[j].clone();
                if particle.position == particle2.position {
                    particle.position.x += 0.01;
                }
                match particle_state {
                    ParticleType::Fluid => {
                        let distance = particle.position - particle2.position;
                        let distance_length = distance.length();
                        let min_distance = particle.radius * 3.0 + particle2.radius * 3.0;
                        if distance_length >= min_distance*1.2 {
                            continue;
                        }
                        let impulse = getinfluence(distance_length/min_distance) * distance.normalize() * 0.2;
                        particle.apply_force(impulse * particle2.mass);
                        particle2.apply_force(-impulse * particle.mass);
                        // let average_velocity = particle.velocity + particle2.velocity / 2.0;
                        // particle.velocity = average_velocity*0.1 + particle.velocity*0.9;
                        // particle2.velocity = average_velocity*0.1 + particle2.velocity*0.9;
                        self.particles[j] = *particle2;
                    }
                    ParticleType::Solid => {
                        let distance = particle.position - particle2.position;
                        let distance_length = distance.length();
                        let min_distance = particle.radius + particle2.radius;
                        if distance_length >= min_distance {
                            continue;
                        }
                        let overlap = min_distance - distance_length;
                        let normal = distance.normalize();
                        let correction = normal * overlap / 2.0;
                        particle.position += correction;
                        particle2.position -= correction;
                        let relative_velocity = particle.velocity - particle2.velocity;
                        let normal_velocity = relative_velocity.dot(normal);
                        if normal_velocity > 0.0 {
                            continue;
                        }
                        let restitution = BOUNCE_DAMPING;
                        let impulse = (normal_velocity * -(1.0 + restitution)) / (1.0 / particle.mass + 1.0 / particle2.mass);
                        let impulse_vector = normal * impulse;
                        particle.velocity += impulse_vector / particle.mass;
                        particle2.velocity -= impulse_vector / particle2.mass;
                        self.particles[j] = *particle2;
                    }
                    ParticleType::Gas => {
                        let distance = particle.position - particle2.position;
                        let distance_length = distance.length();
                        let min_distance = particle.radius + particle2.radius;
                        if distance_length >= min_distance {
                            continue;
                        }
                        let overlap = min_distance - distance_length;
                        let normal = distance.normalize();
                        let correction = normal * overlap / 2.0;
                        particle.position += correction;
                        particle2.position -= correction;
                        let relative_velocity = particle.velocity - particle2.velocity;
                        let normal_velocity = relative_velocity.dot(normal);
                        if normal_velocity > 0.0 {
                            continue;
                        }
                        let restitution = BOUNCE_DAMPING;
                        let impulse = (normal_velocity * -(1.0 + restitution)) / (1.0 / particle.mass + 1.0 / particle2.mass);
                        let impulse_vector = normal * impulse;
                        particle.velocity += impulse_vector / particle.mass;
                        particle2.velocity -= impulse_vector / particle2.mass;
                        self.particles[j] = *particle2;
                    }
                    ParticleType::Plasma => {
                        let distance = particle.position - particle2.position;
                        let distance_length = distance.length();
                        let min_distance = particle.radius + particle2.radius;
                        if distance_length >= min_distance {
                            continue;
                        }
                        let overlap = min_distance - distance_length;
                        let normal = distance.normalize();
                        let correction = normal * overlap / 2.0;
                        particle.position += correction;
                        particle2.position -= correction;
                        let relative_velocity = particle.velocity - particle2.velocity;
                        let normal_velocity = relative_velocity.dot(normal);
                        if normal_velocity > 0.0 {
                            continue;
                        }
                        let restitution = BOUNCE_DAMPING;
                        let impulse = (normal_velocity * -(1.0 + restitution)) / (1.0 / particle.mass + 1.0 / particle2.mass);
                        let impulse_vector = normal * impulse;
                        particle.velocity += impulse_vector / particle.mass;
                        particle2.velocity -= impulse_vector / particle2.mass;
                        self.particles[j] = *particle2;
                    }
                }
            }
            particle.update();
            self.particles[i] = *particle;
        }
    }

    fn update_image(&self) {
        set_camera(&Camera2D {
            render_target: Some(self.render_target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, self.width*PIXEL_SCALE, self.height*PIXEL_SCALE))
        });
        clear_background(self.color);
        for particle in self.particles.iter() {
            particle.draw();
        }
        set_default_camera();
    }
}

#[derive(Clone, Copy, Debug)]
enum ParticleType {
    Fluid,
    Solid,
    Gas,
    Plasma,
}

#[macroquad::main("Particletoy")]
async fn main() {
    let mut particle_state = ParticleType::Fluid;
    let possible_colors = vec![
        RED, GREEN, BLUE, YELLOW, PURPLE, ORANGE, PINK, VIOLET, MAGENTA, LIME, SKYBLUE,
    ];
    // let random_color = possible_colors.choose().unwrap();
    let mut particle_boxes = vec![FluidBox::new(0.0, 0.0, 500.0, 500.0, DARKGRAY)];
    particle_boxes[0].add_particle(FluidParticle::new(Some(Vec2::new(50.0, 50.0)), 1.0, 5.0, WHITE));
    let mut gravity_amount = -0.1;
    if is_key_pressed(KeyCode::G) {
        gravity_amount = -0.1-gravity_amount;
    }
    if is_key_pressed(KeyCode::C) {
        let random_color = possible_colors.choose().unwrap();
        let x = particle_boxes.len() as f32 * 600.0;
        particle_boxes.push(FluidBox::new(x, 0.0, 500.0, 500.0, *random_color));
    }
    if is_key_pressed(KeyCode::L) {
        particle_state = match particle_state {
            ParticleType::Fluid => ParticleType::Solid,
            ParticleType::Solid => ParticleType::Gas,
            ParticleType::Gas => ParticleType::Plasma,
            ParticleType::Plasma => ParticleType::Fluid,
        };
    }
    loop {
        for _i in 0..11 {

        for particle_box in particle_boxes.iter_mut() {
            for particle in particle_box.particles.iter_mut() {
                particle.velocity += Vec2::new(0.0, gravity_amount);
            }
            {
                if is_key_down(KeyCode::Space) {
                    particle_box.add_particle(FluidParticle::new(None, 1.0, 5.0, WHITE));
                }
                if is_key_down(KeyCode::Right) {
                    particle_box.apply_force(Vec2::new(0.1, 0.0));
                }
                if is_key_down(KeyCode::Left) {
                    particle_box.apply_force(Vec2::new(-0.1, 0.0));
                }
                if is_key_down(KeyCode::Up) {
                    particle_box.apply_force(Vec2::new(0.0, 0.1-gravity_amount));
                }
                if is_key_down(KeyCode::Down) {
                    particle_box.apply_force(Vec2::new(0.0, -0.1-gravity_amount));
                }
                if is_key_down(KeyCode::S) {
                    for particle in particle_box.particles.iter_mut() {
                        particle.velocity = Vec2::new(0.0, 0.0);
                    }
                }
                if is_key_down(KeyCode::R) {
                    particle_box.particles.clear();
                }
                if is_key_down(KeyCode::D) {
                    particle_box.particles.pop();
                }
                if is_mouse_button_down(MouseButton::Left) {
                    let mouse_position = mouse_position();
                    let mouse_position = Vec2::new(mouse_position.0, particle_box.height-mouse_position.1);
                    particle_box.add_particle(FluidParticle::new(Some(mouse_position), 5.0, 20.0, LIGHTGRAY));
                }
            }
            particle_box.update(particle_state);
            particle_box.update_image();
            draw_texture_ex(
                &particle_box.render_target.texture,
                particle_box.x_pos,
                particle_box.y_pos,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(particle_box.width, particle_box.height)),
                    ..Default::default()
                },
            );
        }
        }
        next_frame().await
    }
    
}
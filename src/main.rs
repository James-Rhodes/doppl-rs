use std::{f32::consts::PI, time::Duration};

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

// Colors
const PARTICLE_COLOR: Color = Color::GREEN;
const PARTICLE_RADIUS: f32 = 10.;
const PARTICLE_SIZE: Vec3 = Vec2::splat(PARTICLE_RADIUS).extend(1.0);

const PARTICLE_SPAWN_RATE_MS: u64 = 50;

// struct Receiver {}

#[derive(Component)]
struct Transmitter {
    spawn_point: Vec2,
    spawn_rate: Timer,
}

#[derive(Component)]
struct SignalParticle {
    start_pos: Vec2,
    speed: f32,
    amplitude: f32,
    frequency: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (propagate_particle, produce_particle, destroy_particle).chain(),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let start_pos = Vec2::new(500., 200.);
    commands.spawn(Transmitter {
        spawn_point: start_pos,
        spawn_rate: Timer::new(
            Duration::from_millis(PARTICLE_SPAWN_RATE_MS),
            TimerMode::Repeating,
        ),
    });
    let start_pos = Vec2::new(500., -200.);
    commands.spawn(Transmitter {
        spawn_point: start_pos,
        spawn_rate: Timer::new(
            Duration::from_millis(PARTICLE_SPAWN_RATE_MS),
            TimerMode::Repeating,
        ),
    });
}

fn propagate_particle(mut query: Query<(&mut Transform, &SignalParticle)>, time: Res<Time>) {
    let mut cnt = 0;
    for (mut particle_transforms, signal_particle) in query.iter_mut() {
        cnt += 1;
        let t = time.elapsed().as_millis() as f32 / 1000.;
        particle_transforms.translation.x += signal_particle.speed * time.delta_seconds();

        let a = signal_particle.amplitude;
        let k = 2. * PI * signal_particle.frequency / signal_particle.speed; // v = \omega/k =
                                                                             // \lambda/T = \lambda * f
        let x = particle_transforms.translation.x;

        // Classic
        // propagating wave equation
        let f = signal_particle.frequency;
        particle_transforms.translation.y =
            a * f32::sin(k * x - 2. * PI * f * t) + signal_particle.start_pos.y;
    }

    println!("Count: {}", cnt);
}

fn produce_particle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<&mut Transmitter>,
    time: Res<Time>,
) {
    for mut tx in query.iter_mut() {
        tx.spawn_rate.tick(time.delta());

        if tx.spawn_rate.finished() {
            let start_pos = tx.spawn_point;
            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: meshes.add(Circle::default()).into(),
                    material: materials.add(PARTICLE_COLOR),
                    transform: Transform::from_translation(start_pos.extend(0.))
                        .with_scale(PARTICLE_SIZE),
                    ..default()
                },
                SignalParticle {
                    start_pos,
                    amplitude: 100.,
                    speed: -100.,
                    frequency: 1.,
                },
            ));
        }
    }
}

fn destroy_particle(
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform)>,
    query: Query<(Entity, &Transform, &SignalParticle)>,
) {
    let (camera, camera_transform) = camera.single();
    for (entity, transform, _) in query.iter() {
        let particle_pos = transform.translation;

        let world_left_bound = camera
            .viewport_to_world(camera_transform, Vec2::new(0., 0.))
            .unwrap()
            .origin
            .x;

        if particle_pos.x < world_left_bound - PARTICLE_RADIUS {
            commands.entity(entity).despawn();
        }
    }
}

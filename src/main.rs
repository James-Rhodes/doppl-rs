use std::{f32::consts::PI, time::Duration};

use bevy::{prelude::*, sprite::MaterialMesh2dBundle, window::RawHandleWrapper};

// Colors
const PARTICLE_AMPLITUDE: f32 = 50.;
const PARTICLE_COLOR: Color = Color::GREEN;
const PARTICLE_RADIUS: f32 = 5.;
const PARTICLE_SIZE: Vec3 = Vec2::splat(PARTICLE_RADIUS).extend(1.0);
const PARTICLE_SPEED: f32 = -100.;
const PARTICLE_FREQUENCY: f32 = 0.5;

const TRANSMITTER_COLOR: Color = Color::ORANGE;
const TRANSMITTER_SIZE: f32 = 25.;

const RECEIVER_COLOR: Color = Color::RED;
const RECEIVER_WIDTH: f32 = 2. * PARTICLE_AMPLITUDE + 2. * PARTICLE_RADIUS;
const RECEIVER_HEIGHT: f32 = 2. * RECEIVER_WIDTH;
const RECEIVER_SIZE: Vec2 = Vec2::new(RECEIVER_HEIGHT, RECEIVER_WIDTH);
const RECEIVER_TIME_SCALE: f32 = 1.0 / PARTICLE_FREQUENCY;
const RECEIVER_DELTA_X_PER_SECOND: f32 = 2. * RECEIVER_WIDTH / RECEIVER_TIME_SCALE;
const RECEIVER_PLOT_COLOR: Color = Color::BLACK;
const RECEIVER_PLOT_RADIUS: f32 = 10.;
const RECEIVER_PLOT_SIZE: Vec3 = Vec2::splat(RECEIVER_PLOT_RADIUS).extend(1.0);
const RECEIVER_SPEED: f32 = 80.;

const PARTICLE_SPAWN_RATE_MS: u64 = 50;

#[derive(Component, Default)]
struct Receiver {
    prev_collision_time: Option<f32>,
    current_draw_position: f32,
}

#[derive(Component)]
struct Mover;

#[derive(Component, Default)]
struct Transmitter {
    spawn_point: Vec2,
    spawn_rate: Timer,
}

#[derive(Component, Default)]
struct SignalParticle {
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
            (
                propagate_particle,
                produce_particle,
                destroy_particle,
                handle_rx_collision,
                move_rx,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    commands.spawn(Camera2dBundle::default());

    let transmitter_x = 400.;
    let transmitter_y = 200.;
    let half_tri_size = TRANSMITTER_SIZE / 2.;
    let pta = Vec2::new(half_tri_size, half_tri_size);
    let ptb = Vec2::new(0., -half_tri_size);
    let ptc = Vec2::new(-half_tri_size, half_tri_size);
    commands.spawn((
        Transmitter {
            spawn_rate: Timer::new(
                Duration::from_millis(PARTICLE_SPAWN_RATE_MS),
                TimerMode::Repeating,
            ),
            ..Default::default()
        },
        MaterialMesh2dBundle {
            mesh: meshes.add(Triangle2d::new(pta, ptb, ptc)).into(),
            material: materials.add(TRANSMITTER_COLOR),
            transform: Transform::from_xyz(transmitter_x, transmitter_y, 1.),
            ..default()
        },
    ));

    commands.spawn((
        Transmitter {
            spawn_rate: Timer::new(
                Duration::from_millis(PARTICLE_SPAWN_RATE_MS),
                TimerMode::Repeating,
            ),
            ..Default::default()
        },
        MaterialMesh2dBundle {
            mesh: meshes.add(Triangle2d::new(pta, ptb, ptc)).into(),
            material: materials.add(TRANSMITTER_COLOR),
            transform: Transform::from_xyz(transmitter_x, -transmitter_y, 1.),
            ..default()
        },
    ));

    let start_x = -300.;
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(Rectangle::from_size(RECEIVER_SIZE)).into(),
            material: materials.add(RECEIVER_COLOR),
            transform: Transform::from_xyz(start_x, transmitter_y, 1.),
            ..default()
        },
        Receiver::default(),
    ));

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(Rectangle::from_size(RECEIVER_SIZE)).into(),
            material: materials.add(RECEIVER_COLOR),
            transform: Transform::from_xyz(start_x, -transmitter_y, 1.),
            ..default()
        },
        Receiver::default(),
        Mover,
    ));
}

fn propagate_particle(mut query: Query<(&mut Transform, &SignalParticle)>, time: Res<Time>) {
    for (mut particle_transforms, signal_particle) in query.iter_mut() {
        let t = time.elapsed().as_millis() as f32 / 1000.;
        particle_transforms.translation.x += signal_particle.speed * time.delta_seconds();

        let a = signal_particle.amplitude;
        let k = 2. * PI * signal_particle.frequency / signal_particle.speed; // v = \omega/k =
                                                                             // \lambda/T = \lambda * f
        let x = particle_transforms.translation.x;

        // Classic
        // propagating wave equation
        let f = signal_particle.frequency;
        particle_transforms.translation.y = a * f32::sin(k * x - 2. * PI * f * t);
    }
}

fn produce_particle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(Entity, &mut Transmitter)>,
    time: Res<Time>,
) {
    for (tx_entity, mut tx) in query.iter_mut() {
        tx.spawn_rate.tick(time.delta());

        if tx.spawn_rate.finished() {
            let new_particle = commands
                .spawn((
                    MaterialMesh2dBundle {
                        mesh: meshes.add(Circle::default()).into(),
                        material: materials.add(PARTICLE_COLOR),
                        transform: Transform::from_translation(tx.spawn_point.extend(-1.))
                            .with_scale(PARTICLE_SIZE),
                        ..default()
                    },
                    SignalParticle {
                        amplitude: PARTICLE_AMPLITUDE,
                        speed: PARTICLE_SPEED,
                        frequency: PARTICLE_FREQUENCY,
                    },
                ))
                .id();

            commands.entity(tx_entity).add_child(new_particle);
        }
    }
}

fn handle_rx_collision(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    sig_query: Query<(Entity, &GlobalTransform, &Transform), With<SignalParticle>>,
    mut rx_query: Query<(Entity, &Transform, &mut Receiver)>,
    time: Res<Time>,
) {
    for (sig_entity, sig_global_transform, sig_transform) in sig_query.iter() {
        let particle_pos = sig_global_transform.translation().xy();
        for (rx_entity, rx_transform, mut rx) in rx_query.iter_mut() {
            let rx_translation = rx_transform.translation;
            let rx_right_bound = rx_translation.x + RECEIVER_WIDTH;
            let rx_top_bound = rx_translation.y + RECEIVER_HEIGHT / 2.;
            let rx_bottom_bound = rx_translation.y - RECEIVER_HEIGHT / 2.;

            if particle_pos.y < rx_top_bound
                && particle_pos.y > rx_bottom_bound
                && particle_pos.x < rx_right_bound
            {
                let t = time.elapsed().as_millis() as f32 / 1000.;
                let y = sig_transform.translation.y;
                commands.entity(sig_entity).despawn();

                if rx.current_draw_position > 2. * RECEIVER_WIDTH {
                    // If we have already plotted over the entire width of the receiver then just
                    // don't do anything
                    commands.entity(rx_entity).remove::<Mover>();
                    continue;
                }

                let plot_point = commands
                    .spawn(MaterialMesh2dBundle {
                        mesh: meshes.add(Circle::default()).into(),
                        material: materials.add(RECEIVER_PLOT_COLOR),
                        transform: Transform::from_xyz(
                            (RECEIVER_WIDTH) - rx.current_draw_position,
                            y,
                            2.,
                        )
                        .with_scale(RECEIVER_PLOT_SIZE),
                        ..default()
                    })
                    .id();

                commands.entity(rx_entity).add_child(plot_point);

                if rx.prev_collision_time.is_none() {
                    rx.prev_collision_time = Some(t);
                }
                rx.current_draw_position +=
                    RECEIVER_DELTA_X_PER_SECOND * (t - rx.prev_collision_time.unwrap());

                rx.prev_collision_time = Some(t);
            }
        }
    }
}

fn destroy_particle(
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform)>,
    sig_query: Query<(Entity, &GlobalTransform), With<SignalParticle>>,
) {
    let (camera, camera_transform) = camera.single();
    for (entity, transform) in sig_query.iter() {
        let particle_pos = transform.translation();

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

fn move_rx(mut rx_query: Query<&mut Transform, (With<Receiver>, With<Mover>)>, time: Res<Time>) {
    for mut transform in rx_query.iter_mut() {
        transform.translation.x += RECEIVER_SPEED * time.delta_seconds();
    }
}

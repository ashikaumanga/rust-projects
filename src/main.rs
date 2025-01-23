use bevy::{color::palettes::css::{SILVER, WHITE}, pbr::wireframe::{Wireframe, WireframeConfig, WireframePlugin}, prelude::*, render::{mesh::{self, VertexAttributeValues}, settings::{RenderCreation, WgpuFeatures, WgpuSettings}, RenderPlugin}, utils::info};
//use bevy_rts_camera::{RtsCamera, RtsCameraControls, RtsCameraPlugin}
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use noise::{BasicMulti, NoiseFn, Perlin, Seedable};

#[derive(Component)]
struct Shape;

#[derive(Component)]
struct Ship {
    location: Vec3,
    orientation: Quat,
    forward_speed: f32,   // Speed along the forward direction
    pitch_input: f32,     // Pitch input
    roll_input: f32,      // Roll input
    yaw_input: f32,       // Yaw input
}

#[derive(Component)]
struct FollowCamera;

#[derive(Component)]
struct Terrain;

use std::f32::consts::PI;
const MAX_SPEED: f32 = 20.0;
const ACCELERATION: f32 = 0.1;
const PITCH_SPEED: f32 = 1.5;
const ROLL_SPEED: f32 = 1.9;
const YAW_SPEED: f32 = 1.25;
const INPUT_RESPONSE: f32 = 8.0;


fn main() {
    App::new()
    .add_plugins((
        DefaultPlugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                // WARN this is a native only feature. It will not work with webgl or webgpu
                features: WgpuFeatures::POLYGON_MODE_LINE,
                ..default()
            }),
            ..default()
        }),
        WireframePlugin,
    ))
    .insert_resource(WireframeConfig {
        global: false,
        default_color: WHITE.into(),
    })
    .add_plugins(PanOrbitCameraPlugin)
    .add_systems(Startup, (setup_scene,setup_ship))
    .add_systems(Update, (update_cube,input_controls_ship,update_ship_physics,sync_ship_transform,toggle_wireframe,camera_follow) )
    .run();
}

fn update_cube(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

fn sync_ship_transform(mut query: Query<(&Ship, &mut Transform)>) {
    let rotation_fix = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2*2.0);

    for (ship, mut transform) in query.iter_mut() {
        transform.translation = ship.location;  
        transform.rotation =     ship.orientation * rotation_fix;
        //transform.rotate_y(std::f32::consts::FRAC_PI_2*2.0);
    }
}

fn toggle_wireframe(mut commands: Commands,
                    landscape_wireframes: Query<Entity,(With<Terrain>, With<Wireframe>)>,
                    langscapes: Query<Entity,(With<Terrain>,Without<Wireframe>)>,
                    input: Res<ButtonInput<KeyCode>>,) {
                        if input.just_pressed(KeyCode::Space) {
                            info!("Space pressed");
                            for terrain in &langscapes {
                                info!("Adding wireframe");
                                commands.entity(terrain).insert(Wireframe);
                            }
                            for terrain in &landscape_wireframes {
                                info!("Removing wireframe");
                                commands.entity(terrain).remove::<Wireframe>();
                            }
                        }
}

fn update_ship_physics(
    time: Res<Time>,
    mut query: Query<&mut Ship>,

) {
    let delta = time.delta_secs();

    for mut ship in query.iter_mut() {
        let rotation_fix = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2*2.0);
        // Update orientation based on pitch, roll, and yaw inputs
        let rotation = Quat::from_rotation_z(ship.roll_input * ROLL_SPEED * delta)
            * Quat::from_rotation_x(ship.pitch_input * PITCH_SPEED * delta)
            * Quat::from_rotation_y(ship.yaw_input * YAW_SPEED * delta);

        ship.orientation = (ship.orientation * rotation).normalize();

        // Update position based on forward direction and speed
        let forward = ship.orientation * Vec3::Z * -1.0; // Forward is negative Z
        
        let t = ship.forward_speed;
        ship.location += forward * t * delta;
    }
}



fn input_controls_ship(mut query: Query<&mut Ship>, time: Res<Time>, input: Res<ButtonInput<KeyCode>>) {
    let delta = time.delta_secs();
   
    for mut ship in query.iter_mut() {
        if input.pressed(KeyCode::KeyW) {
            ship.forward_speed = (ship.forward_speed + ACCELERATION * time.delta_secs()).max(MAX_SPEED);
        }
        if input.pressed(KeyCode::KeyS) {
            ship.forward_speed = (ship.forward_speed - ACCELERATION * time.delta_secs()).min(0.0);
        }

         // Smooth input interpolation for pitch, roll, and yaw
         ship.pitch_input = lerp(
            ship.pitch_input,
            axis_input(&input, KeyCode::ArrowUp, KeyCode::ArrowDown),
            INPUT_RESPONSE * delta,
        );
        /* 
        ship.roll_input = lerp(
            ship.roll_input,
            axis_input(&input, KeyCode::ArrowRight, KeyCode::ArrowLeft),
            INPUT_RESPONSE * delta,
        );
        */

        ship.yaw_input = lerp(
            ship.yaw_input,
            axis_input(&input, KeyCode::ArrowRight, KeyCode::ArrowLeft),
            INPUT_RESPONSE * delta,
        );
    
        //ship.roll_input = ship.yaw_input * 0.2;
    }
}

fn setup_ship(mut commands: Commands, asset_server: Res<AssetServer>) {
    
    let assest = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("executioner.gltf")
        );
        
    //commands.spawn(SceneRoot(assest)).insert(Ship);
    commands.spawn(Ship {
        location: Vec3::ZERO,
        orientation: Quat::IDENTITY,
        forward_speed: 0.0,
            pitch_input: 0.0,
            roll_input: 0.0,
            yaw_input: 0.0,
    }).insert(Transform::default())
    .insert(GlobalTransform::default())
    .insert(Name::new("Spaceship"))
    .insert(SceneRoot(assest));
}

fn setup_scene(mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
       ) {

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),Shape
    ));


    //terrain
    let mut terrain = Mesh::from(Plane3d::default().mesh().size(500.0, 500.0).subdivisions(100));
    let terrain_height = 20.0;
    let noise = BasicMulti::<Perlin>::default();
    let p = terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION);
    if let Some(VertexAttributeValues::Float32x3(pos)) = p {
        //let perlin = Perlin::new(1);
        for i in 0..pos.len() {
            let val = noise.get([pos[i][0] as f64/ 200., pos[i][2] as f64 / 200.]);

            //dbg!(val);
            pos[i][1] = val as f32 * terrain_height;
        }
    }
    terrain.compute_normals();
    commands.spawn((
        Mesh3d(meshes.add(terrain)),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
        Terrain
    ));


    // light
    // directional 'sun' light
     commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        }));

    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        PanOrbitCamera::default()
    )).insert(FollowCamera);

}

/// System to make the camera follow the ship
fn camera_follow(
    query_ship: Query<&Ship>,
    mut query_camera: Query<&mut Transform, With<FollowCamera>>,
) {
    if let Ok(ship) = query_ship.get_single() {
        if let Ok(mut camera_transform) = query_camera.get_single_mut() {
            // Camera offset behind and above the ship
            let offset = Vec3::new(0.0, -5.0, -30.0);
            let target_position = ship.location + (ship.orientation * -offset);

            // Smoothly move the camera to the target position
            camera_transform.translation = target_position;

            // Make the camera look at the ship
            camera_transform.look_at(ship.location, Vec3::Y);
        }
    }
}

/// Helper functions
fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + t * (end - start)
}

/// Helper function for axis input handling
fn axis_input(input: &Res<ButtonInput<KeyCode>>, negative: KeyCode, positive: KeyCode) -> f32 {
    let mut value = 0.0;
    if input.pressed(negative) {
        value -= 1.0;
    }
    if input.pressed(positive) {
        value += 1.0;
    }
    value
}
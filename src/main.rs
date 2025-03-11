use bevy::{color::palettes::css::{SILVER, WHITE}, pbr::wireframe::{Wireframe, WireframeConfig, WireframePlugin}, prelude::*, render::{mesh::{self, VertexAttributeValues}, settings::{RenderCreation, WgpuFeatures, WgpuSettings}, RenderPlugin}, scene::ron::de, utils::info};
//use bevy_rts_camera::{RtsCamera, RtsCameraControls, RtsCameraPlugin}
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use noise::{BasicMulti, NoiseFn, Perlin};
mod mesh_utils;

#[derive(Component)]
struct Shape;

#[derive(Component)]
struct Enemy {
    position: Vec3,
    velocity: Vec3,
}

#[derive(Component)]
struct Player {
    position: Vec3,
    orientation: Quat,
    forward_speed: f32,   // Speed along the forward direction
    pitch_input: f32,     // Pitch input
    yaw_input: f32,       // Yaw input
    bank : f32
}
#[derive(Debug)]
enum CameraMode {
    FPV,
    TPV,
    PAN
}

#[derive(Resource)]

struct GameSettings {
    camera_mode: CameraMode,
    enemy_count: u32,
       //forces
   cohesion_force:   f32,
   separation_force: f32,
   alignment_force: f32,
   //distances
   separation_distance: f32,
   alignment_distance: f32,
   cohesion_distance: f32,
   max_speed: f32,
}

#[derive(Component)]
struct FollowCamera;

#[derive(Component)]
struct Terrain;

use std::{collections::HashMap, f32::consts::PI};
const MAX_SPEED: f32 = 1000.0;
const ACCELERATION: f32 = 20.0;
const PITCH_SPEED: f32 = 1.5;
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
    .insert_resource(GameSettings {
        camera_mode: CameraMode::TPV,
        enemy_count: 10,
        cohesion_force: 0.02,
        separation_force: 0.08,
        alignment_force: 0.06,
        separation_distance: 10.0,
        alignment_distance: 10.0,
        cohesion_distance: 15.0,
        max_speed: 10.0,
    })
    .add_plugins(PanOrbitCameraPlugin)
    .add_systems(Startup, (setup_scene,setup_ship ,setup_enemy))
    .add_systems(Update, (input_controls_ship,enemy_ai,switch_camera_mode, update_cube,update_ship_physics,sync_player_mesh_transform,sync_enemy_mesh_transform,toggle_wireframe,update_camera) )
    .run();
}

fn update_cube(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

/* 
fn sync_ship_mesh_transform(mut query: Query<(&Ship, &mut Transform)>) {
    let mesh_rotation_fix_y = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2*2.0);
    
    for (ship, mut transform) in query.iter_mut() {
        let banking_rotation_z = Quat::from_rotation_z(ship.bank);

        //transform.com
       
        transform.rotation =    ship.orientation * banking_rotation_z*mesh_rotation_fix_y;
        transform.translation = ship.location; 

    }
}*/

fn sync_enemy_mesh_transform(
    mut query: Query<(&Enemy, &mut Transform)>,
) {
    for (enemy, mut transform) in query.iter_mut() {
        transform.translation = enemy.position;
        let up = Vec3::Y;
        let forward = enemy.velocity.normalize();
        let rotation = Quat::from_rotation_arc(up, forward);
        transform.rotation = rotation;
    }
}
fn sync_player_mesh_transform(
    mut query: Query<(&Player, &mut Transform, &mut Visibility)>,
    mut meshes: ResMut<Assets<Mesh>>,
    game_settings: ResMut<GameSettings>) {
    let mesh_rotation_fix_y = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2*2.0);
    
    for (ship, mut transform, mut visibility) in query.iter_mut() {
        let banking_rotation_z = Quat::from_rotation_z(ship.bank);

        //transform.com
       
        transform.rotation =    ship.orientation * banking_rotation_z*mesh_rotation_fix_y;
        transform.translation = ship.position;

        *visibility = match game_settings.camera_mode {
            CameraMode::FPV => Visibility::Hidden,
            CameraMode::TPV => Visibility::Visible,
            CameraMode::PAN => Visibility::Visible,
            
        };

    }
}

fn toggle_wireframe(mut commands: Commands,
                    landscape_wireframes: Query<Entity,(With<Terrain>, With<Wireframe>)>,
                    langscapes: Query<Entity,(With<Terrain>,Without<Wireframe>)>,
                    input: Res<ButtonInput<KeyCode>>,) {
                        if input.just_pressed(KeyCode::Space) {
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

fn update_ship_physics2(
    time: Res<Time>,
    mut query: Query<&mut Player>,

) {
    let delta = time.delta_secs();
    for mut ship in query.iter_mut() {
        // Update orientation based on pitch, yaw inputs
        let rotation = //Quat::from_rotation_z(ship.roll_input * ROLL_SPEED * delta)
              Quat::from_rotation_x(ship.pitch_input * PITCH_SPEED * delta)
            * Quat::from_rotation_y(ship.yaw_input * YAW_SPEED * delta);

        ship.orientation = (ship.orientation * rotation).normalize();

        // Update position based on forward direction and speed
        let forward = ship.orientation * Vec3::Z * -1.0; // Forward is negative Z
        
        let t = ship.forward_speed;
        ship.position += forward *t * delta;
    }
}

fn update_ship_physics(
    time: Res<Time>,
    mut query: Query<&mut Player>,
) {
    let delta = time.delta_secs();
    for mut ship in query.iter_mut() {
        // Update orientation based on pitch, yaw inputs
        let yaw_rotation = Quat::from_rotation_y(ship.yaw_input * YAW_SPEED * delta);
        let pitch_rotation = Quat::from_rotation_x(ship.pitch_input * PITCH_SPEED * delta);
        // Combine rotations: yaw around global Y axis, pitch around local X axis
        ship.orientation = (yaw_rotation * ship.orientation * pitch_rotation).normalize();

        // Update position based on forward direction and speed
        let forward = ship.orientation * Vec3::Z * -1.0; // Forward is negative Z
        let t = ship.forward_speed;
        ship.position += forward * t * delta;
    }
}


fn switch_camera_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut game_settings: ResMut<GameSettings>,
) {
    if input.just_pressed(KeyCode::KeyV) {
        match game_settings.camera_mode {
            CameraMode::FPV => game_settings.camera_mode = CameraMode::TPV,
            CameraMode::TPV => game_settings.camera_mode = CameraMode::PAN,
            CameraMode::PAN => game_settings.camera_mode = CameraMode::FPV,
        }
        info!("Switched camera mode to {:?}", game_settings.camera_mode);
    }
}

fn enemy_ai(mut query_enemies: Query<(Entity,&mut Enemy,&mut Transform)>,
            player_query: Query<&Player>,
            game_settings: ResMut<GameSettings>,
            time: Res<Time>) {
    let player = player_query.single();
     //distances
     let separation_distance = game_settings.separation_distance;
     let alignment_distance = game_settings.alignment_distance;
     let cohesion_distance = game_settings.cohesion_distance;
     //forces
     let cohesion_force = game_settings.cohesion_force;
     let seperation_force = game_settings.separation_force;
     let alignment_force = game_settings.alignment_force;
     //max speed
     let max_speed = game_settings.max_speed;


     let fleet_target = player.position;

     struct Body {
        position: Vec3,
        velocity: Vec3,
    }

    let mut body_map: HashMap<u32, Body> = HashMap::new();
    for (entity, enemy,  _) in query_enemies.iter() {
        body_map.insert(entity.index(), Body {
            position: enemy.position,
            velocity: enemy.velocity,
        });
    }
    for (entity, enemy, _) in query_enemies.iter() {
        let mut separation = Vec3::ZERO;
        let mut alignment = Vec3::ZERO;
        let mut cohesion = Vec3::ZERO;
        let mut boundary_force = Vec3::ZERO;
        let mut fleet_force;
        let mut neighbor_count = 0;

        for (other_entity, other_enemy,_) in query_enemies.iter() {
            if entity.index() != other_entity.index() {
                let distance = enemy.position.distance(other_enemy.position);

                if distance < separation_distance && distance > 0.0 {
                    let diff= enemy.position - other_enemy.position;
                    separation += diff.normalize() / distance;
                    neighbor_count += 1;
                }
                if distance < alignment_distance {
                    alignment += other_enemy.velocity;
                    neighbor_count += 1;
                }
                if distance < cohesion_distance {
                    cohesion += other_enemy.position;
                    neighbor_count += 1;
                }
            }
        }
        if neighbor_count > 0 {
            separation /= neighbor_count as f32;
            alignment /= neighbor_count as f32;
            cohesion /= neighbor_count as f32;

            cohesion = (cohesion - enemy.position);
            if cohesion.length() > 0.0 {
                cohesion = cohesion.normalize() * max_speed - enemy.velocity;
                cohesion = cohesion.clamp_length_max(cohesion_force);
            }
            if alignment.length() > 0.0 {
                alignment = alignment.normalize() * max_speed - enemy.velocity;
                alignment = alignment.clamp_length_max(alignment_force);
            }
            if separation.length() > 0.0 {
                separation = separation.normalize() * max_speed - enemy.velocity;
                separation = separation.clamp_length_max(seperation_force);
            }
        }

        fleet_force = (fleet_target - enemy.position);
        if fleet_force.length() > 0.0 {
            fleet_force = fleet_force.normalize() * max_speed - enemy.velocity;
            fleet_force = fleet_force.clamp_length_max(0.01);
        }
        let tmp = body_map.get_mut(&entity.index()).unwrap();
        tmp.velocity += (separation + alignment + cohesion + fleet_force)*2.0;
        //tmp.velocity = tmp.velocity.clamp_length_max(max_speed);
        tmp.position += tmp.velocity * time.delta_secs();
    }
    //sync back
    for (entity, mut enemy,   _) in query_enemies.iter_mut() {
        let tmp =body_map.get(&entity.index()).unwrap();
        enemy.position = tmp.position;
        enemy.velocity = tmp.velocity;
        
    }

           
}

fn input_controls_ship(mut query: Query<&mut Player>, time: Res<Time>, input: Res<ButtonInput<KeyCode>>) {
    let delta = time.delta_secs();
   
    for mut ship in query.iter_mut() {
        if input.pressed(KeyCode::KeyW) {
            ship.forward_speed = (ship.forward_speed + ACCELERATION * time.delta_secs()).min(MAX_SPEED);
            
        }
        if input.pressed(KeyCode::KeyS) {
            ship.forward_speed = (ship.forward_speed - ACCELERATION * time.delta_secs()).max(0.0);
        }

         // Smooth input interpolation for pitch and yaw
         ship.pitch_input = lerp(
            ship.pitch_input,
            axis_input(&input, KeyCode::ArrowUp, KeyCode::ArrowDown),
            INPUT_RESPONSE * delta,
        );
          
        ship.yaw_input = lerp(
            ship.yaw_input,
            axis_input(&input, KeyCode::ArrowRight, KeyCode::ArrowLeft),
            INPUT_RESPONSE * delta,
        );
    
        //only used for Rendering the mesh
        
        ship.bank = lerp(
            ship.bank,
            axis_input(&input, KeyCode::ArrowRight, KeyCode::ArrowLeft),
            3.0 * delta,
        );
        
    }
}

fn setup_enemy(mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_settings : ResMut<GameSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

){
    
    //let assest = asset_server.load(
     //   GltfAssetLabel::Scene(0).from_asset("omen.gltf")
     //   );
     //let sr =   SceneRoot(assest);
     let enemy_mesh = Mesh3d(meshes.add(Cone::new(1.0, 2.0)));
     let enemy_mat = MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255)));
 

     for i in 0..game_settings.enemy_count {
        let x = (i as f32 * 10.0).sin() * 20.0;
        let z = (i as f32 * 10.0).cos() * 20.0;
        commands.spawn(Enemy {
            position: Vec3::new(x, 10.0, z),
            velocity: Vec3::ZERO,
        }).insert(Transform::default())
        .insert(GlobalTransform::default())
        .insert(enemy_mesh.clone())
        .insert(enemy_mat.clone());
    
}
}
fn setup_ship(mut commands: Commands,
    asset_server: Res<AssetServer>) {
    
    let assest = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("executioner.gltf")
        );

     //let aroot =   SceneRoot(assest);    
    //commands.spawn(SceneRoot(assest)).insert(Ship);
    commands.spawn(Player {
        position: Vec3::ZERO,
        orientation: Quat::IDENTITY,
        forward_speed: 0.0,
            pitch_input: 0.0,
            yaw_input: 0.0,
            bank: 0.0
    }).insert(Transform::default())
    .insert(GlobalTransform::default())
    .insert(Name::new("Player"))
    .insert(SceneRoot(assest));
}

fn setup_scene(mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
       ) {

    // cube
    //commands.spawn((
   //     Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
   //     MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
   //     Transform::from_xyz(0.0, 0.5, 0.0),Shape
  //  ));


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


    // lights
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
fn update_camera(
    query_ship: Query<&Player>,
    mut query_camera: Query<&mut Transform, With<FollowCamera>>,
    game_settings: Res<GameSettings>
) {
    match game_settings.camera_mode {
        CameraMode::FPV => update_camera_fpv(query_ship, query_camera),
        CameraMode::TPV => update_camera_tpv(query_ship, query_camera),
        CameraMode::PAN => {},
    }
    /* 
    if let Ok(ship) = query_ship.get_single() {
        if let Ok(mut camera_transform) = query_camera.get_single_mut() {
            // Camera offset behind and above the ship
            let offset = Vec3::new(0.0, -5.0, -30.0);
            let target_position = ship.location + (ship.orientation * -offset);

            // Smoothly move the camera to the target position
            camera_transform.translation = camera_transform.translation.lerp(target_position, 0.2);
            // Make the camera look at the ship
            camera_transform.look_at(ship.location, Vec3::Y);
            
        }
    }
    */
}
fn update_camera_fpv(
    query_ship: Query<&Player>,
    mut query_camera: Query<&mut Transform, With<FollowCamera>>,
) {
    if let Ok(ship) = query_ship.get_single() {
        if let Ok(mut camera_transform) = query_camera.get_single_mut() {
            // Camera offset behind and above the ship
            let offset = Vec3::new(0.0, 0.0, -20.0);
            let target_position = ship.position + (ship.orientation * offset);

            // Smoothly move the camera to the target position
            camera_transform.translation = ship.position; //camera_transform.translation.lerp(target_position, 0.2);

            // Make the camera look at the ships forward direction
            camera_transform.look_at(target_position, Vec3::Y);
            camera_transform.rotate_local_z(ship.bank);

            
        }
    }
}
fn update_camera_tpv(
    query_ship: Query<&Player>,
    mut query_camera: Query<&mut Transform, With<FollowCamera>>,
) {
    if let Ok(ship) = query_ship.get_single() {
        if let Ok(mut camera_transform) = query_camera.get_single_mut() {
            // Camera offset behind and above the ship
            let offset = Vec3::new(0.0, -5.0, -30.0);
            let target_position = ship.position + (ship.orientation * -offset);

            // Smoothly move the camera to the target position
            camera_transform.translation = camera_transform.translation.lerp(target_position, 0.2);
            // Make the camera look at the ship
            camera_transform.look_at(ship.position, Vec3::Y);
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
        value = -std::f32::consts::FRAC_PI_2/2.0;
    }
    if input.pressed(positive) {
        value = std::f32::consts::FRAC_PI_2/2.0;
    }
    value
}
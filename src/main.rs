#![allow(unused)] // silence unused warnings while exploring (to comment out)

use bevy::{math::Vec3Swizzles, transform};
use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use bevy::window::PrimaryWindow;
use components::{
	Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, Movable,
	Player, SpriteSize, Velocity,
};
use enemy::EnemyPlugin;
use player::PlayerPlugin;
use std::collections::HashSet;
use std::fmt::format;

mod components;
mod enemy;
mod player;

// region:    --- Asset Constants

const PLAYER_SPRITE: &str = "player_a_01.png";
const PLAYER_SIZE: (f32, f32) = (144., 75.);
const PLAYER_LASER_SPRITE: &str = "laser_a_01.png";
const PLAYER_LASER_SIZE: (f32, f32) = (9., 54.);

const ENEMY_SPRITE: &str = "enemy_a_01.png";
const ENEMY_SIZE: (f32, f32) = (144., 75.);
const ENEMY_LASER_SPRITE: &str = "laser_b_01.png";
const ENEMY_LASER_SIZE: (f32, f32) = (17., 55.);

const EXPLOSION_SHEET: &str = "explo_a_sheet.png";
const EXPLOSION_LEN: usize = 16;

const SPRITE_SCALE: f32 = 0.5;

const FONT_FAMILY: &str = "Roboto-Light.ttf";

// endregion: --- Asset Constants

// region:    --- Game Constants

const TIME_STEP: f32 = 1. / 60.;
const BASE_SPEED: f32 = 500.;

const PLAYER_RESPAWN_DELAY: f64 = 2.;
const ENEMY_MAX: u32 = 2;
const FORMATION_MEMBERS_MAX: u32 = 2;

// endregion: --- Game Constants

// region:    --- Resources
#[derive(Resource)]
pub struct WinSize {
	pub w: f32,
	pub h: f32,
}

#[derive(Resource)]
struct GameTextures {
	player: Handle<Image>,
	player_laser: Handle<Image>,
	enemy: Handle<Image>,
	enemy_laser: Handle<Image>,
	explosion: Handle<TextureAtlas>,
}

#[derive(Resource)]
struct EnemyCount(u32);


#[derive(Component)]
struct ScoreText;

// A unit struct to help identify the color-changing Text component
#[derive(Component)]
struct NLivesText;

#[derive(Resource, Debug)]
struct PlayerState {
	on: bool,       // alive
	last_shot: f64, // -1 if not shot
	score: f64,
	n_lives: u16,
}
impl Default for PlayerState {
	fn default() -> Self {
		Self {
			on: false,
			last_shot: -1.,
			score: 0.,
			n_lives: 3,
		}
		
	}
}

impl PlayerState {
	pub fn shot(&mut self, time: f64) {
		self.on = false;
		self.last_shot = time;
	}
	pub fn spawned(&mut self) {
		self.on = true;
		self.last_shot = -1.;
	}

	pub fn add_score(&mut self) {
		self.score += 100.;
	}
}
// endregion: --- Resources

fn main() {
	App::new()
		.insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Rust Invaders!".into(),
				resolution: (598., 676.).into(),
				..Default::default()
			}),
			..Default::default()
		}))
		.add_plugin(PlayerPlugin)
		.add_plugin(EnemyPlugin)
		.add_startup_system(setup_system)
		.add_system(movable_system)
		.add_system(player_laser_hit_enemy_system)
		.add_system(add_text_to_screen)
		.add_system(enemy_laser_hit_player_system)
		.add_system(explosion_to_spawn_system)
		.add_system(explosion_animation_system)
		.add_system(spawn_game_over)
		.run();
}

fn setup_system(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
	query: Query<&Window, With<PrimaryWindow>>,
	mut player_state: ResMut<PlayerState>,

) {
	// camera
	commands.spawn(Camera2dBundle::default());

	// capture window size
	let Ok(primary) = query.get_single() else {
        return;
    };
	let (win_w, win_h) = (primary.width(), primary.height());

	// position window (for tutorial)
	// window.set_position(IVec2::new(2780, 4900));

	// add WinSize resource
	let win_size = WinSize { w: win_w, h: win_h };
	commands.insert_resource(win_size);

	// create explosion texture atlas
	let texture_handle = asset_server.load(EXPLOSION_SHEET);
	let texture_atlas =
		TextureAtlas::from_grid(texture_handle, Vec2::new(64., 64.), 4, 4, None, None);
	let explosion = texture_atlases.add(texture_atlas);

	// add GameTextures resource
	let game_textures = GameTextures {
		player: asset_server.load(PLAYER_SPRITE),
		player_laser: asset_server.load(PLAYER_LASER_SPRITE),
		enemy: asset_server.load(ENEMY_SPRITE),
		enemy_laser: asset_server.load(ENEMY_LASER_SPRITE),
		explosion,
	};
	// score text 

	commands.spawn((
		TextBundle::from_section(
			"Score: ", 
			TextStyle { 
				font: asset_server.load(
					FONT_FAMILY), 
					font_size: 50.0, 
					color: Color::WHITE,
					..default()
				}
			),
	ScoreText	
	),);

	// lives text
	commands.spawn((
		TextBundle::from_section(
			format!("# Lives {}",player_state.n_lives),
			TextStyle { 
				font: asset_server.load(
					FONT_FAMILY), 
					font_size: 50.0, 
					color: Color::WHITE,
					..default()
				}
			).with_style(Style {
				position_type: PositionType::Absolute,
				position: UiRect {
					top: Val::Px(5.0),
					right: Val::Px(15.0),
					..default()
				},
				..default()
			}),
			NLivesText	
	),);


	commands.insert_resource(game_textures);
	commands.insert_resource(EnemyCount(0));
}

fn spawn_game_over(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut player_state: ResMut<PlayerState>,
	win_size: Res<WinSize>,
) {
	if(player_state.n_lives == 0){
		// simple for now
		commands.spawn((
			TextBundle::from_section(
				"GAME OVER!",
				TextStyle { 
					font: asset_server.load(
						FONT_FAMILY), 
						font_size: 100.0, 
						color: Color::WHITE,
						..default()
					}
				).with_text_alignment(TextAlignment::Center)
				.with_style(Style {
					position_type: PositionType::Absolute,
					position: UiRect {
						top: Val::Px(win_size.h/2.),
						left: Val::Px(win_size.w/6.),
						..default()
					},
					..default()
				}),
		),);

	}
}

fn add_text_to_screen(
	mut score_text_query: Query<&mut Text,With<ScoreText>>,
	//mut n_lives_text_query: Query<&mut Text, With<NLivesText>>,
	mut player_state: ResMut<PlayerState>,
){

	for mut text in &mut score_text_query {
		text.sections[0].value = format!("Score: {}",player_state.score);
	} 

	
}

fn movable_system(
	mut commands: Commands,
	win_size: Res<WinSize>,
	mut query: Query<(Entity, &Velocity, &mut Transform, &Movable)>,
) {
	for (entity, velocity, mut transform, movable) in query.iter_mut() {
		let translation = &mut transform.translation;
		translation.x += velocity.x * TIME_STEP * BASE_SPEED;
		translation.y += velocity.y * TIME_STEP * BASE_SPEED;


		if movable.auto_despawn {
			// despawn when out of screen
			const MARGIN: f32 = 200.;
			if translation.y > win_size.h / 2. + MARGIN
				|| translation.y < -win_size.h / 2. - MARGIN
				|| translation.x > win_size.w / 2. + MARGIN
				|| translation.x < -win_size.w / 2. - MARGIN
			{
				commands.entity(entity).despawn();
			}
		}
	}
}

#[allow(clippy::type_complexity)] // for the Query types.
fn player_laser_hit_enemy_system(
	mut commands: Commands,
	mut enemy_count: ResMut<EnemyCount>,
	mut player_state: ResMut<PlayerState>,
	laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromPlayer>)>,
	enemy_query: Query<(Entity, &Transform, &SpriteSize), With<Enemy>>,
) {
	let mut despawned_entities: HashSet<Entity> = HashSet::new();
	
	// iterate through the lasers
	for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
		if despawned_entities.contains(&laser_entity) {
			continue;
		}

		let laser_scale = laser_tf.scale.xy();

		// iterate through the enemies
		for (enemy_entity, enemy_tf, enemy_size) in enemy_query.iter() {
			if despawned_entities.contains(&enemy_entity)
				|| despawned_entities.contains(&laser_entity)
			{
				continue;
			}

			let enemy_scale = enemy_tf.scale.xy();

			// determine if collision
			let collision = collide(
				laser_tf.translation,
				laser_size.0 * laser_scale,
				enemy_tf.translation,
				enemy_size.0 * enemy_scale,
			);

			// perform collision
			if collision.is_some() {
				// remove the enemy
				commands.entity(enemy_entity).despawn();
				despawned_entities.insert(enemy_entity);
				enemy_count.0 -= 1;
				player_state.add_score();

				// remove the laser
				commands.entity(laser_entity).despawn();
				despawned_entities.insert(laser_entity);

				// spawn the explosionToSpawn
				commands.spawn(ExplosionToSpawn(enemy_tf.translation));
			}
		}
	}
}

#[allow(clippy::type_complexity)] // for the Query types.
fn enemy_laser_hit_player_system(
	mut commands: Commands,
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromEnemy>)>,
	player_query: Query<(Entity, &Transform, &SpriteSize), With<Player>>,
	mut n_lives_text_query: Query<&mut Text, With<NLivesText>>,
) {
	if let Ok((player_entity, player_tf, player_size)) = player_query.get_single() {
		let player_scale = player_tf.scale.xy();

		for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
			let laser_scale = laser_tf.scale.xy();

			// determine if collision
			let collision = collide(
				laser_tf.translation,
				laser_size.0 * laser_scale,
				player_tf.translation,
				player_size.0 * player_scale,
			);

			// perform the collision
			if collision.is_some() {
				// remove the player
				commands.entity(player_entity).despawn();
				player_state.shot(time.elapsed_seconds_f64());

				// remove the laser
				commands.entity(laser_entity).despawn();

				// restar lives 

				player_state.n_lives  -= 1;
				for mut text in &mut n_lives_text_query {
					text.sections[0].value = format!("# Lives {}",player_state.n_lives)
				}  

				// spawn the explosionToSpawn
				commands.spawn(ExplosionToSpawn(player_tf.translation));

				break;
			}
		}
	}
}

fn explosion_to_spawn_system(
	mut commands: Commands,
	game_textures: Res<GameTextures>,
	query: Query<(Entity, &ExplosionToSpawn)>,
) {
	for (explosion_spawn_entity, explosion_to_spawn) in query.iter() {
		// spawn the explosion sprite
		commands
			.spawn(SpriteSheetBundle {
				texture_atlas: game_textures.explosion.clone(),
				transform: Transform {
					translation: explosion_to_spawn.0,
					..Default::default()
				},
				..Default::default()
			})
			.insert(Explosion)
			.insert(ExplosionTimer::default());

		// despawn the explosionToSpawn
		commands.entity(explosion_spawn_entity).despawn();
	}
}

fn explosion_animation_system(
	mut commands: Commands,
	time: Res<Time>,
	mut query: Query<(Entity, &mut ExplosionTimer, &mut TextureAtlasSprite), With<Explosion>>,
) {
	for (entity, mut timer, mut sprite) in query.iter_mut() {
		timer.0.tick(time.delta());
		if timer.0.finished() {
			sprite.index += 1; // move to next sprite cell
			if sprite.index >= EXPLOSION_LEN {
				commands.entity(entity).despawn()
			}
		}
	}
}

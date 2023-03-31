#![windows_subsystem = "windows"]
mod bricks;
mod consts;

use bevy::prelude::*;
use bevy_utils::Duration;
use bricks::{Board, Brick, BrickView, Dot};
use consts::*;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Playing,
    GameOver,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tetris".to_string(),
                resizable: false,
                resolution: (360., 443.).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(GameData::default())
        .add_startup_system(setup_screen.in_base_set(StartupSet::PreStartup))
        .add_state::<GameState>()
        .add_system(new_game_system.in_schedule(OnEnter(GameState::Playing)))
        .add_systems(
            (
                keyboard_system,
                move_brick_system,
                freeze_brick_system,
                scoreboard_system,
            )
                .in_set(OnUpdate(GameState::Playing)),
        )
        .add_system(game_over_setup.in_schedule(OnEnter(GameState::GameOver)))
        .add_system(game_over_system.in_set(OnUpdate(GameState::GameOver)))
        .run();
}

fn setup_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: asset_server.load("screen.png"),
        ..default()
    });
    commands
        .spawn(init_text(
            "000000",
            TEXT_SCORE_X,
            TEXT_SCORE_Y,
            &asset_server,
        ))
        .insert(ScoreText);
    commands
        .spawn(init_text(
            "000000",
            TEXT_LINES_X,
            TEXT_LINES_Y,
            &asset_server,
        ))
        .insert(LinesText);
    commands
        .spawn(init_text("00", TEXT_LEVEL_X, TEXT_LEVEL_Y, &asset_server))
        .insert(LevelText);
}

#[derive(Component)]
struct DotBundle;
#[derive(Component)]
struct BoardBundle;

#[derive(Component)]
struct BrickBoardBundle;

#[derive(Component)]
struct BrickNextBundle;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct LinesText;

#[derive(Component)]
struct LevelText;
#[derive(Component)]
struct GameOverText;

/// keyboard_system only handle keyboard input
/// won't handle tick-tick falling
fn keyboard_system(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut game: ResMut<GameData>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform), With<BrickBoardBundle>>,
) {
    let Ok((moving_entity, mut transform)) = query.get_single_mut() else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::Up) {
        let rotated = game.moving_brick.rotate();
        if game.board.valid_brick(&rotated, &game.moving_pos) {
            spawn_brick_board(&mut commands, rotated.into(), game.moving_pos);
            game.moving_brick = rotated;
            commands.entity(moving_entity).despawn_recursive();
        }
    }
    if keyboard_input.just_pressed(KeyCode::Space) {
        let mut valid = false;
        while game
            .board
            .valid_brick(&game.moving_brick, &game.moving_pos.down())
        {
            game.moving_pos.move_down();
            transform.translation.y -= consts::DOT_WIDTH_PX;
            valid = true;
        }
        if valid {
            // Trigger the falling timer immediately to spawn a new brick right away
            let elapse = game.falling_timer.duration() - Duration::from_nanos(1);
            game.falling_timer.set_elapsed(elapse);
        }
    }

    // Speed up the falling timer when the down key is pressed
    if keyboard_input.just_pressed(KeyCode::Down) {
        let level = game.level;
        game.falling_timer
            .set_duration(Duration::from_secs_f32(get_speed(level) / 10.));
    }
    if keyboard_input.just_released(KeyCode::Down) {
        let level = game.level;
        game.falling_timer
            .set_duration(Duration::from_secs_f32(get_speed(level)));
    }

    let just_pressed = keyboard_input.any_just_pressed(vec![KeyCode::Left, KeyCode::Right]);
    if just_pressed {
        game.keyboard_timer.reset();
    }
    let ticked = game.keyboard_timer.tick(time.delta()).finished();
    if !ticked && !just_pressed {
        return;
    }

    if keyboard_input.pressed(KeyCode::Left)
        && game
            .board
            .valid_brick(&game.moving_brick, &game.moving_pos.left())
    {
        game.moving_pos.move_left();
        transform.translation.x -= consts::DOT_WIDTH_PX;
    }

    if keyboard_input.pressed(KeyCode::Right)
        && game
            .board
            .valid_brick(&game.moving_brick, &game.moving_pos.right())
    {
        game.moving_pos.move_right();
        transform.translation.x += consts::DOT_WIDTH_PX;
    }
}

/// move_brick_system only handle tick-tick falling
/// won't handle keyboard input
fn move_brick_system(
    //commands: Commands,
    mut game: ResMut<GameData>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<BrickBoardBundle>>,
) {
    let ticked = game.falling_timer.tick(time.delta()).finished();
    let Ok(mut transform) = query.get_single_mut() else {
        return;
    };
    if !ticked {
        return;
    }

    if game
        .board
        .valid_brick(&game.moving_brick, &game.moving_pos.down())
    {
        //after ticking, brick falling one line.
        game.moving_pos.move_down();
        transform.translation.y -= consts::DOT_WIDTH_PX;
    } else {
        //there is no space to fall, so freeze the brick.
        let frozen_brick = game.moving_brick;
        let frozen_pos = game.moving_pos;
        game.board.occupy_brick(&frozen_brick, &frozen_pos);
        game.freeze = true;
        //if we destroy moving brick here.
        //there is flash, when destroy brick ,and re-draw board.
        //commands.entity(entity).despawn_recursive();
    }
}
fn freeze_brick_system(
    mut commands: Commands,
    mut game: ResMut<GameData>,
    mut brick: Query<Entity, With<BrickBoardBundle>>,
    mut board: Query<Entity, With<BoardBundle>>,
) {
    if !game.freeze {
        return;
    }

    //step 1. check: we can clean one line?
    game.deleted_lines = game.board.clean_lines();

    //destroy moving brick
    if let Ok(entity) = brick.get_single_mut() {
        commands.entity(entity).despawn_recursive();
    }
    //destroy board
    if let Ok(entity) = board.get_single_mut() {
        commands.entity(entity).despawn_recursive();
    }
    //redraw board
    spawn_board(&mut commands, &game.board);
}

#[allow(clippy::type_complexity)]
fn scoreboard_system(
    mut commands: Commands,
    mut state: ResMut<NextState<GameState>>,
    mut game: ResMut<GameData>,
    mut next_brick: Query<Entity, With<BrickNextBundle>>,
    mut query: ParamSet<(
        Query<&mut Text, With<ScoreText>>,
        Query<&mut Text, With<LinesText>>,
        Query<&mut Text, With<LevelText>>,
    )>,
) {
    if game.deleted_lines > 0 {
        game.score += get_score(game.level, game.deleted_lines);
        game.lines += game.deleted_lines;
        game.deleted_lines = 0;

        let level = get_level(game.lines);
        if game.level != level {
            game.level = level;
            game.falling_timer
                .set_duration(Duration::from_secs_f32(get_speed(level)));
        }
        if let Ok(mut text) = query.p0().get_single_mut() {
            text.sections[0].value = format!("{:06}", game.score);
        }
        if let Ok(mut text) = query.p1().get_single_mut() {
            text.sections[0].value = format!("{:06}", game.lines);
        }
        if let Ok(mut text) = query.p2().get_single_mut() {
            text.sections[0].value = format!("{:02}", game.level);
        }
    }

    //next moving brick
    //step 1. generate new brick(using next_brick, and rand generate new next_brick)
    //game.freeze = false;
    if !game.freeze {
        return;
    }

    game.freeze = false;
    game.score += SCORE_PER_DROP;
    if let Ok(mut text) = query.p0().get_single_mut() {
        text.sections[0].value = format!("{:06}", game.score);
    }

    game.moving_pos = consts::BRICK_START_DOT;
    game.moving_brick = game.next_brick;
    game.next_brick = Brick::rand();

    if game.board.valid_brick(&game.moving_brick, &BRICK_START_DOT) {
        //step 2.2 destroy next_brick
        if let Ok(entity) = next_brick.get_single_mut() {
            commands.entity(entity).despawn_recursive();
        }

        //step 3.1 draw new one in start point
        spawn_brick_board(
            &mut commands,
            game.moving_brick.into(),
            consts::BRICK_START_DOT,
        );
        //step 3.3 draw new next_brick
        spawn_brick_next(&mut commands, game.next_brick.into());
        // Reset the timer
        game.falling_timer.reset();
    } else {
        //game over!
        state.set(GameState::GameOver);
    }
}

fn game_over_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut board: Query<Entity, With<BoardBundle>>,
    mut next_brick: Query<Entity, With<BrickNextBundle>>,
) {
    //destroy board
    if let Ok(entity) = board.get_single_mut() {
        commands.entity(entity).despawn_recursive();
    }
    //destroy next brick
    if let Ok(entity) = next_brick.get_single_mut() {
        commands.entity(entity).despawn_recursive();
    }
    //show GameOver
    commands
        .spawn(init_text(
            STRING_GAME_OVER,
            TEXT_GAME_X,
            TEXT_GAME_Y,
            &asset_server,
        ))
        .insert(GameOverText);
}
fn game_over_system(
    mut commands: Commands,
    mut state: ResMut<NextState<GameState>>,
    mut game: ResMut<GameData>,
    mut game_over: Query<Entity, With<GameOverText>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if !keyboard_input.pressed(KeyCode::Space) {
        return;
    }

    game.reset();

    if let Ok(entity) = game_over.get_single_mut() {
        commands.entity(entity).despawn_recursive();
    }
    state.set(GameState::Playing);
}

#[allow(clippy::type_complexity)]
fn new_game_system(
    mut commands: Commands,
    game: ResMut<GameData>,
    mut query: ParamSet<(
        Query<&mut Text, With<ScoreText>>,
        Query<&mut Text, With<LinesText>>,
        Query<&mut Text, With<LevelText>>,
    )>,
) {
    let moving_brick = game.moving_brick;
    let next_brick = game.next_brick;
    spawn_brick_board(&mut commands, moving_brick.into(), BRICK_START_DOT);
    spawn_brick_next(&mut commands, next_brick.into());

    if let Ok(mut text) = query.p0().get_single_mut() {
        text.sections[0].value = format!("{:06}", game.score);
    }
    if let Ok(mut text) = query.p1().get_single_mut() {
        text.sections[0].value = format!("{:06}", game.lines);
    }
    if let Ok(mut text) = query.p2().get_single_mut() {
        text.sections[0].value = format!("{:02}", game.level);
    }
}

fn spawn_brick_next(commands: &mut Commands, brick: BrickView) {
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_xyz(
                consts::NEXT_BRICK_LEFT_PX - consts::WINDOWS_WIDTH / 2.0,
                consts::NEXT_BRICK_BOTTOM_PX - consts::WINDOWS_HEIGHT / 2.0,
                0.0,
            ),
            ..default()
        })
        .insert(BrickNextBundle)
        .with_children(|parent| {
            (0..4).for_each(|i| {
                spawn_dot_as_child(parent, dot_to_vec2(&brick.dots[i]));
            });
        });
}

fn spawn_board(commands: &mut Commands, board: &Board) {
    commands
        .spawn(SpriteBundle {
            //from middle pixel to pixel of (left,bottom)
            transform: Transform::from_xyz(
                10.0 - consts::WINDOWS_WIDTH / 2.0 + consts::BOARD_LEFT_PX,
                10.0 - consts::WINDOWS_HEIGHT / 2.0 + consts::BOARD_BOTTOM_PX,
                0.0, //zero,which one pixel behind the UI-screen png; cannot be seen in screen
            ),
            ..default()
        })
        .insert(BoardBundle)
        .with_children(|parent| {
            (0..consts::BOARD_X)
                .flat_map(|a| (0..consts::BOARD_Y).map(move |b| Dot(a, b)))
                .filter(|dot| board.occupied_dot(dot))
                .for_each(|dot| spawn_dot_as_child(parent, dot_to_vec2(&dot)));
        });
}

fn spawn_brick_board(commands: &mut Commands, brick: BrickView, dot_in_board: Dot) {
    commands
        .spawn(SpriteBundle {
            //from middle pixel to pixel of (left,bottom)
            transform: Transform::from_xyz(
                dot_in_board.0 as f32 * consts::DOT_WIDTH_PX + 10.0 - consts::WINDOWS_WIDTH / 2.0
                    + consts::BOARD_LEFT_PX,
                dot_in_board.1 as f32 * consts::DOT_WIDTH_PX + 10.0 - consts::WINDOWS_HEIGHT / 2.0
                    + consts::BOARD_BOTTOM_PX,
                0.0, //zero,which one pixel behind the UI-screen png; cannot be seen in screen
            ),
            ..default()
        })
        .insert(BrickBoardBundle)
        .with_children(|parent| {
            (0..4).for_each(|i| {
                spawn_dot_as_child(parent, dot_to_vec2(&brick.dots[i]));
            });
        });
}
fn spawn_dot_as_child(commands: &mut ChildBuilder, trans: Vec2) {
    commands
        .spawn(sprit_bundle(20., Color::BLACK, trans))
        .with_children(|parent| {
            parent
                .spawn(sprit_bundle(16., consts::BACKGROUND, Vec2::default()))
                .with_children(|parent| {
                    parent.spawn(sprit_bundle(12., Color::BLACK, Vec2::default()));
                });
        });
}

#[inline]
fn sprit_bundle(width: f32, color: Color, trans: Vec2) -> SpriteBundle {
    SpriteBundle {
        transform: Transform {
            translation: Vec3::new(trans.x, trans.y, 0.1),
            ..default()
        },
        sprite: Sprite {
            color,
            custom_size: Some(Vec2::new(width, width)),
            ..default()
        },
        ..default()
    }
}
#[inline]
fn init_text(msg: &str, x: f32, y: f32, asset_server: &Res<AssetServer>) -> TextBundle {
    // scoreboard
    TextBundle {
        text: Text::from_section(
            msg,
            TextStyle {
                font: asset_server.load("digital7mono.ttf"),
                font_size: 16.0,
                color: Color::BLACK,
            }, //Default::default(),
        ),
        style: Style {
            align_self: AlignSelf::FlexEnd,
            position_type: PositionType::Absolute,
            position: UiRect {
                left: Val::Px(x),
                top: Val::Px(y),
                ..default()
            },
            ..default()
        },
        ..default()
    }
}

#[derive(Resource)]
pub struct GameData {
    board: Board,
    moving_brick: Brick,
    moving_pos: Dot,
    next_brick: Brick,
    freeze: bool,
    deleted_lines: u32,
    score: u32,
    lines: u32,
    level: u32,
    keyboard_timer: Timer,
    falling_timer: Timer,
}

impl GameData {
    fn reset(&mut self) {
        self.board.clear();
        self.freeze = false;
        self.deleted_lines = 0;
        self.score = 0;
        self.lines = 0;
        self.level = 0;
        self.keyboard_timer = Timer::from_seconds(consts::TIMER_KEY_SECS, TimerMode::Repeating);
        self.falling_timer = Timer::from_seconds(consts::TIMER_FALLING_SECS, TimerMode::Repeating);
    }
}
impl Default for GameData {
    fn default() -> Self {
        Self {
            board: Board::default(),
            moving_brick: Brick::rand(),
            moving_pos: consts::BRICK_START_DOT,
            next_brick: Brick::rand(),
            freeze: false,
            keyboard_timer: Timer::from_seconds(consts::TIMER_KEY_SECS, TimerMode::Repeating),
            falling_timer: Timer::from_seconds(consts::TIMER_FALLING_SECS, TimerMode::Repeating),
            deleted_lines: 0,
            score: 0,
            lines: 0,
            level: 0,
        }
    }
}

#[inline]
fn dot_to_vec2(dot: &Dot) -> Vec2 {
    Vec2::new(DOT_WIDTH_PX * dot.0 as f32, DOT_WIDTH_PX * dot.1 as f32)
}

/// delay = 725 * .85 ^ level + level (ms)
///
/// use formula from dwhacks, http://gist.github.com/dwhacks/8644250
#[inline]
pub fn get_speed(level: u32) -> f32 {
    consts::TIMER_FALLING_SECS * (0.85_f32).powi(level as i32) + level as f32 / 1000.0
}

/// use as [Original Nintendo Scoring System]
///
/// https://tetris.fandom.com/wiki/Scoring
#[inline]
pub fn get_score(level: u32, erase_lines: u32) -> u32 {
    assert!(0 < erase_lines);
    assert!(erase_lines <= 4);
    vec![40, 100, 300, 1200][(erase_lines - 1) as usize] * (level + 1)
}

/// increase level every 10 lines.
#[inline]
pub fn get_level(total_lines: u32) -> u32 {
    (total_lines / 10).min(99)
}

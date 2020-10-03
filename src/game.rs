use std::collections::{HashMap, HashSet};

use euclid::{
    default::{Point2D, Transform2D, Vector2D},
    point2, vec2,
};

use crate::{
    constants::{SCREEN_SIZE, TICK_DT, ZOOM_LEVEL},
    gl,
    graphics::{load_image, render_sprite, Sprite, Vertex, TEXTURE_ATLAS_SIZE},
    input::{InputEvent, Key},
    level::{create_level, generate_tile_buffer, DoorTile, Level, Tile, TILE_SIZE},
    texture_atlas::{TextureAtlas, TextureRect},
};

pub struct Game {
    program: gl::Program,
    ground_buffer: gl::VertexBuffer,
    vertex_buffer: gl::VertexBuffer,
    images: Images,

    tick: usize,
    rewind: bool,
    level: Level,
    controls: Controls,

    player: Ghost,
    old_players: Vec<Ghost>,
    buttons: HashMap<Point2D<i32>, Button>,
    doors: HashMap<Point2D<i32>, Door>,
}

impl Game {
    pub fn new(gl_context: &mut gl::Context) -> Self {
        let vertex_shader = unsafe {
            gl_context
                .create_shader(gl::ShaderType::Vertex, include_str!("shaders/shader.vert"))
                .unwrap()
        };
        let fragment_shader = unsafe {
            gl_context
                .create_shader(
                    gl::ShaderType::Fragment,
                    include_str!("shaders/shader.frag"),
                )
                .unwrap()
        };

        let mut program = unsafe {
            gl_context
                .create_program(&gl::ProgramDescriptor {
                    vertex_shader: &vertex_shader,
                    fragment_shader: &fragment_shader,
                    uniforms: &[
                        gl::UniformEntry {
                            name: "u_transform",
                            ty: gl::UniformType::Mat3,
                        },
                        gl::UniformEntry {
                            name: "u_texture",
                            ty: gl::UniformType::Texture,
                        },
                    ],
                    vertex_format: gl::VertexFormat {
                        stride: std::mem::size_of::<Vertex>(),
                        attributes: &[
                            gl::VertexAttribute {
                                name: "a_pos",
                                ty: gl::VertexAttributeType::Float,
                                size: 2,
                                offset: 0,
                            },
                            gl::VertexAttribute {
                                name: "a_uv",
                                ty: gl::VertexAttributeType::Float,
                                size: 2,
                                offset: 2 * 4,
                            },
                            gl::VertexAttribute {
                                name: "a_color",
                                ty: gl::VertexAttributeType::Float,
                                size: 4,
                                offset: 4 * 4,
                            },
                        ],
                    },
                })
                .unwrap()
        };

        let transform = Transform2D::create_scale(
            1.0 / SCREEN_SIZE.width as f32,
            1.0 / SCREEN_SIZE.height as f32,
        )
        .post_scale(2., 2.)
        .post_scale(ZOOM_LEVEL, ZOOM_LEVEL)
        .post_scale(TILE_SIZE as f32, TILE_SIZE as f32)
        .post_translate(vec2(-1.0, -1.0));
        program
            .set_uniform(
                0,
                gl::Uniform::Mat3([
                    [transform.m11, transform.m12, 0.0],
                    [transform.m21, transform.m22, 0.0],
                    [transform.m31, transform.m32, 1.0],
                ]),
            )
            .unwrap();

        let mut texture = unsafe {
            gl_context
                .create_texture(
                    gl::TextureFormat::RGBAFloat,
                    TEXTURE_ATLAS_SIZE.width,
                    TEXTURE_ATLAS_SIZE.height,
                )
                .unwrap()
        };
        let mut atlas = TextureAtlas::new((TEXTURE_ATLAS_SIZE.width, TEXTURE_ATLAS_SIZE.height));
        program
            .set_uniform(1, gl::Uniform::Texture(&texture))
            .unwrap();

        let vertex_buffer = unsafe { gl_context.create_vertex_buffer().unwrap() };

        let images = unsafe {
            Images {
                ghost: load_image(
                    include_bytes!("../assets/player.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                ghost_shadow: load_image(
                    include_bytes!("../assets/ghost_shadow.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                ground: load_image(
                    include_bytes!("../assets/ground.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                walls: load_image(
                    include_bytes!("../assets/walls.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                door_h: load_image(
                    include_bytes!("../assets/door.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                door_v: load_image(
                    include_bytes!("../assets/door_v.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                button: load_image(
                    include_bytes!("../assets/button.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
            }
        };

        let level = create_level();
        let ground_buffer = generate_tile_buffer(&level, images.ground, images.walls, gl_context);

        let mut buttons = HashMap::new();
        for (position, button_tile) in level.buttons.iter() {
            buttons.insert(
                *position,
                Button::new(
                    images.button,
                    *position,
                    button_tile.connection.expect("unconnected button"),
                ),
            );
        }

        let mut doors = HashMap::new();
        for (position, door_tile) in level.doors.iter() {
            doors.insert(
                *position,
                Door::new(
                    match door_tile {
                        &DoorTile::Horizontal => images.door_h,
                        &DoorTile::Vertical => images.door_v,
                    },
                    *position,
                ),
            );
        }

        let player = Ghost::new(images.ghost, images.ghost_shadow, level.player_start);
        Self {
            program,
            ground_buffer,
            vertex_buffer,
            images,

            tick: 0,
            rewind: false,
            level,
            controls: Controls::default(),

            player,
            old_players: Vec::new(),
            buttons,
            doors,
        }
    }

    pub fn update(&mut self, inputs: &[InputEvent]) {
        for input in inputs {
            match input {
                InputEvent::KeyDown(Key::W) => {
                    self.controls.up = true;
                }
                InputEvent::KeyUp(Key::W) => {
                    self.controls.up = false;
                }
                InputEvent::KeyDown(Key::A) => {
                    self.controls.left = true;
                }
                InputEvent::KeyUp(Key::A) => {
                    self.controls.left = false;
                }
                InputEvent::KeyDown(Key::S) => {
                    self.controls.down = true;
                }
                InputEvent::KeyUp(Key::S) => {
                    self.controls.down = false;
                }
                InputEvent::KeyDown(Key::D) => {
                    self.controls.right = true;
                }
                InputEvent::KeyUp(Key::D) => {
                    self.controls.right = false;
                }
                _ => {}
            }
        }

        // only current player gets new inputs
        if self.rewind {
            self.tick = self.tick.saturating_sub(5);

            if self.tick == 0 {
                self.rewind = false;

                // move current player into old players list, and create a new player
                let mut old_player = std::mem::replace(
                    &mut self.player,
                    Ghost::new(
                        self.images.ghost,
                        self.images.ghost_shadow,
                        self.level.player_start,
                    ),
                );
                old_player.set_color([1.0, 1.0, 1.0, 0.5]);
                self.old_players.push(old_player);

                for old_player in self.old_players.iter_mut() {
                    old_player.reset(self.level.player_start);
                }
            }
        } else {
            self.player.push_controls(self.controls);

            // all players are updated
            self.player.update(self.tick, &self.level, &self.doors);
            for old_player in self.old_players.iter_mut() {
                old_player.update(self.tick, &self.level, &self.doors);
            }

            let mut players_spatial: HashSet<Point2D<i32>> = HashSet::new();
            players_spatial.insert(point2(
                self.player.position(self.tick).x.floor() as i32,
                self.player.position(self.tick).y.floor() as i32,
            ));
            for old_player in self.old_players.iter() {
                players_spatial.insert(point2(
                    old_player.position(self.tick).x.floor() as i32,
                    old_player.position(self.tick).y.floor() as i32,
                ));
            }
            for button in self.buttons.values_mut() {
                button.update(&players_spatial, &mut self.doors);
            }

            self.tick += 1;
            if self.tick >= LOOP_TICKS {
                self.rewind = true;
            }
        }
    }

    pub fn draw(&mut self, context: &mut gl::Context) {
        let mut vertices = Vec::new();

        for button in self.buttons.values() {
            button.draw(&mut vertices);
        }
        for door in self.doors.values() {
            door.draw(&mut vertices);
        }

        // draw all shadows first
        for old_player in self.old_players.iter() {
            old_player.draw_shadow(self.tick, &mut vertices);
        }
        self.player.draw_shadow(self.tick, &mut vertices);

        // then players
        for old_player in self.old_players.iter() {
            old_player.draw(self.tick, &mut vertices);
        }
        self.player.draw(self.tick, &mut vertices);

        unsafe {
            self.vertex_buffer.write(&vertices);

            context.clear([0., 0., 0., 1.]);

            self.program.render_vertices(&self.ground_buffer).unwrap();
            self.program.render_vertices(&self.vertex_buffer).unwrap();
        }
    }
}

struct Images {
    ghost: TextureRect,
    ghost_shadow: TextureRect,
    ground: TextureRect,
    walls: TextureRect,
    door_h: TextureRect,
    door_v: TextureRect,
    button: TextureRect,
}

#[derive(Default, Clone, Copy)]
struct Controls {
    up: bool,
    left: bool,
    down: bool,
    right: bool,
}

struct Ghost {
    sprite: Sprite,
    shadow: Sprite,
    controls: Vec<Controls>,
    positions: Vec<Point2D<f32>>,
    animation_timer: f32,
}

impl Ghost {
    pub fn new(image: TextureRect, shadow: TextureRect, position: Point2D<f32>) -> Self {
        let mut sprite = Sprite::new(image, GHOST_ANIMATION_FRAMES, point2(6., -4.0));
        let mut shadow = Sprite::new(shadow, 1, point2(6., 3.));

        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        shadow.set_transform(transform);

        Self {
            sprite,
            shadow,
            controls: Vec::new(),
            positions: vec![position],
            animation_timer: 0.,
        }
    }

    pub fn reset(&mut self, position: Point2D<f32>) {
        self.positions = vec![position];
        self.animation_timer = 0.;
    }

    pub fn push_controls(&mut self, controls: Controls) {
        self.controls.push(controls);
    }

    pub fn position(&self, tick: usize) -> Point2D<f32> {
        *self
            .positions
            .get(tick + 1)
            .unwrap_or(self.positions.last().expect("positions vec is empty"))
    }

    pub fn update(&mut self, tick: usize, level: &Level, doors: &HashMap<Point2D<i32>, Door>) {
        if let Some(controls) = self.controls.get(tick) {
            let mut dir: Vector2D<f32> = vec2(0., 0.);
            if controls.up {
                dir.y += 1.;
            }
            if controls.down {
                dir.y -= 1.;
            }
            if controls.right {
                dir.x += 1.;
            }
            if controls.left {
                dir.x -= 1.;
            }

            if dir.length() > 0. {
                // This is the laziest collision detection and resolution in the history of video gam
                let new_pos = *self.positions.last().expect("position vec is empty")
                    + dir.normalize() * GHOST_SPEED * TICK_DT;

                let mut colliding = false;
                let new_pos_tile = point2(new_pos.x.floor() as i32, new_pos.y.floor() as i32);
                if level.tile(new_pos_tile.x, new_pos_tile.y) == Tile::Wall {
                    colliding = true;
                }
                if doors
                    .get(&new_pos_tile)
                    .map(|door| !door.is_open())
                    .unwrap_or(false)
                {
                    colliding = true;
                }

                if !colliding {
                    self.positions.push(new_pos);
                } else {
                    self.positions
                        .push(*self.positions.last().expect("position vec is empty"));
                }
            }
        }

        self.animation_timer = (self.animation_timer + TICK_DT) % GHOST_ANIMATION_TIME;
    }

    pub fn draw_shadow(&self, tick: usize, out: &mut Vec<Vertex>) {
        let position = *self
            .positions
            .get(tick + 1)
            .unwrap_or(self.positions.last().expect("positions vec is empty"));
        render_sprite(&self.shadow, 0, position, out);
    }

    pub fn draw(&self, tick: usize, out: &mut Vec<Vertex>) {
        let frame = (self.animation_timer / GHOST_ANIMATION_TIME * GHOST_ANIMATION_FRAMES as f32)
            .floor() as usize;
        let position = *self
            .positions
            .get(tick + 1)
            .unwrap_or(self.positions.last().expect("positions vec is empty"));
        render_sprite(&self.sprite, frame, position, out);
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.sprite.set_color(color);
    }
}

struct Button {
    sprite: Sprite,
    position: Point2D<i32>,
    connection: Point2D<i32>,
    active: bool,
}

impl Button {
    pub fn new(image: TextureRect, position: Point2D<i32>, connection: Point2D<i32>) -> Self {
        let mut sprite = Sprite::new(image, 2, point2(0., 0.));
        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        Self {
            sprite,
            position,
            connection,
            active: false,
        }
    }

    pub fn update(
        &mut self,
        players_spatial: &HashSet<Point2D<i32>>,
        doors: &mut HashMap<Point2D<i32>, Door>,
    ) {
        if players_spatial.contains(&self.position) {
            self.active = true;
            doors.get_mut(&self.connection).unwrap().open = true;
        } else {
            self.active = false;
            doors.get_mut(&self.connection).unwrap().open = false;
        }
    }

    pub fn draw(&self, out: &mut Vec<Vertex>) {
        render_sprite(
            &self.sprite,
            if self.active { 1 } else { 0 },
            self.position.to_f32(),
            out,
        );
    }
}

struct Door {
    sprite: Sprite,
    position: Point2D<i32>,
    open: bool,
}

impl Door {
    pub fn new(image: TextureRect, position: Point2D<i32>) -> Self {
        let mut sprite = Sprite::new(image, 2, point2(0., 0.));
        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        Self {
            sprite,
            position,
            open: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn draw(&self, out: &mut Vec<Vertex>) {
        render_sprite(
            &self.sprite,
            if self.open { 1 } else { 0 },
            self.position.to_f32(),
            out,
        );
    }
}

// Time loops over 300 ticks, 5 seconds
const LOOP_TICKS: usize = 300;

const GHOST_SPEED: f32 = 5.;

const GHOST_ANIMATION_FRAMES: u32 = 6;
const GHOST_ANIMATION_TIME: f32 = 0.5;

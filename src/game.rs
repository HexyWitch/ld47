use euclid::{
    default::{Box2D, Point2D, Rect, Size2D, Transform2D, Vector2D},
    point2, size2, vec2,
};

use crate::{
    constants::{SCREEN_SIZE, TICK_DT, ZOOM_LEVEL},
    gl,
    graphics::{load_image, render_sprite, Sprite, Vertex, TEXTURE_ATLAS_SIZE},
    input::{InputEvent, Key},
    texture_atlas::{TextureAtlas, TextureRect},
};

pub struct Game {
    program: gl::Program,
    ground_buffer: gl::VertexBuffer,
    vertex_buffer: gl::VertexBuffer,
    images: Images,

    tick: usize,
    rewind: bool,
    controls: Controls,

    player: Ghost,
    old_players: Vec<Ghost>,
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
            }
        };

        let ground_buffer = generate_tile_buffer(images.ground, images.walls, gl_context);

        let player = Ghost::new(images.ghost, images.ghost_shadow, PLAYER_START);
        Self {
            program,
            ground_buffer,
            vertex_buffer,
            images,

            tick: 0,
            rewind: false,
            controls: Controls::default(),

            player,
            old_players: Vec::new(),
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
                    Ghost::new(self.images.ghost, self.images.ghost_shadow, PLAYER_START),
                );
                old_player.set_color([1.0, 1.0, 1.0, 0.5]);
                self.old_players.push(old_player);

                for old_player in self.old_players.iter_mut() {
                    old_player.reset(PLAYER_START);
                }
            }
        } else {
            self.player.push_controls(self.controls);

            // all players are updated
            self.player.update(self.tick);
            for old_player in self.old_players.iter_mut() {
                old_player.update(self.tick);
            }

            self.tick += 1;
            if self.tick >= LOOP_TICKS {
                self.rewind = true;
            }
        }
    }

    pub fn draw(&mut self, context: &mut gl::Context) {
        let mut vertices = Vec::new();

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
        Self {
            sprite: Sprite::new(image, GHOST_ANIMATION_FRAMES, point2(6., -4.0)),
            shadow: Sprite::new(shadow, 1, point2(6., 3.)),
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

    pub fn update(&mut self, tick: usize) {
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
                self.positions.push(
                    *self.positions.last().expect("position vec is empty")
                        + dir.normalize() * GHOST_SPEED * TICK_DT,
                );
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

const LEVEL_HEIGHT: usize = 22;
const LEVEL: [&'static str; LEVEL_HEIGHT] = [
    "########################################",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#        ##########                    #",
    "#        ##########                    #",
    "#        ##########                    #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "#                                      #",
    "########################################",
];

const TILE_SIZE: u32 = 16;

pub fn generate_tile_buffer(
    floor: TextureRect,
    walls: TextureRect,
    context: &mut gl::Context,
) -> gl::VertexBuffer {
    let tile_size = Size2D::new((floor[2] - floor[0]) as f32, (floor[3] - floor[1]) as f32);
    let mut vertices = Vec::new();

    for y_tile in 0..LEVEL_HEIGHT {
        for x_tile in 0..LEVEL[y_tile].len() {
            let tile = match LEVEL[y_tile].chars().nth(x_tile) {
                Some(' ') => floor,
                Some('#') => {
                    let wall_at = |x: i32, y: i32| -> bool {
                        let x = x_tile as i32 + x;
                        let y = y_tile as i32 + y;
                        if x >= 0 && y >= 0 && y < LEVEL_HEIGHT as i32 {
                            LEVEL[y as usize].chars().nth(x as usize) == Some('#')
                        } else {
                            true
                        }
                    };
                    // all different kinds of wall tiles
                    let tl = wall_at(-1, -1);
                    let t = wall_at(0, -1);
                    let tr = wall_at(1, -1);
                    let l = wall_at(-1, 0);
                    let r = wall_at(1, 0);
                    let bl = wall_at(-1, 1);
                    let b = wall_at(0, 1);
                    let br = wall_at(1, 1);

                    if t && r && !tr {
                        [
                            walls[0] + 0 * TILE_SIZE,
                            walls[1] + 2 * TILE_SIZE,
                            walls[0] + 1 * TILE_SIZE,
                            walls[1] + 3 * TILE_SIZE,
                        ]
                    } else if t && l && !tl {
                        [
                            walls[0] + 2 * TILE_SIZE,
                            walls[1] + 2 * TILE_SIZE,
                            walls[0] + 3 * TILE_SIZE,
                            walls[1] + 3 * TILE_SIZE,
                        ]
                    } else if b && r && !br {
                        [
                            walls[0] + 0 * TILE_SIZE,
                            walls[1] + 0 * TILE_SIZE,
                            walls[0] + 1 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                        ]
                    } else if b && l && !bl {
                        [
                            walls[0] + 2 * TILE_SIZE,
                            walls[1] + 0 * TILE_SIZE,
                            walls[0] + 3 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                        ]
                    } else if !t && !l {
                        [
                            walls[0] + 3 * TILE_SIZE,
                            walls[1] + 0 * TILE_SIZE,
                            walls[0] + 4 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                        ]
                    } else if !t && !r {
                        [
                            walls[0] + 4 * TILE_SIZE,
                            walls[1] + 0 * TILE_SIZE,
                            walls[0] + 5 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                        ]
                    } else if !t {
                        [
                            walls[0] + 1 * TILE_SIZE,
                            walls[1] + 2 * TILE_SIZE,
                            walls[0] + 2 * TILE_SIZE,
                            walls[1] + 3 * TILE_SIZE,
                        ]
                    } else if !b && !l {
                        [
                            walls[0] + 3 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                            walls[0] + 4 * TILE_SIZE,
                            walls[1] + 2 * TILE_SIZE,
                        ]
                    } else if !b && !r {
                        [
                            walls[0] + 4 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                            walls[0] + 5 * TILE_SIZE,
                            walls[1] + 2 * TILE_SIZE,
                        ]
                    } else if !b {
                        [
                            walls[0] + 1 * TILE_SIZE,
                            walls[1] + 0 * TILE_SIZE,
                            walls[0] + 2 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                        ]
                    } else if !l {
                        [
                            walls[0] + 2 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                            walls[0] + 3 * TILE_SIZE,
                            walls[1] + 2 * TILE_SIZE,
                        ]
                    } else if !r {
                        [
                            walls[0] + 0 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                            walls[0] + 1 * TILE_SIZE,
                            walls[1] + 2 * TILE_SIZE,
                        ]
                    } else {
                        [
                            walls[0] + 1 * TILE_SIZE,
                            walls[1] + 1 * TILE_SIZE,
                            walls[0] + 2 * TILE_SIZE,
                            walls[1] + 2 * TILE_SIZE,
                        ]
                    }
                }
                Some(c) => {
                    panic!("unknown tile type {}", c);
                }
                None => {
                    continue;
                }
            };

            let y_tile = LEVEL_HEIGHT - 1 - y_tile;
            let tile_rect = Box2D::new(
                point2(
                    x_tile as f32 * tile_size.width,
                    y_tile as f32 * tile_size.height,
                ),
                point2(
                    (x_tile + 1) as f32 * tile_size.width,
                    (y_tile + 1) as f32 * tile_size.height,
                ),
            );
            let uv_pos = point2(
                tile[0] as f32 / TEXTURE_ATLAS_SIZE.width as f32,
                tile[1] as f32 / TEXTURE_ATLAS_SIZE.height as f32,
            );
            let uv_size = size2(
                (tile[2] - tile[0]) as f32 / TEXTURE_ATLAS_SIZE.width as f32,
                (tile[3] - tile[1]) as f32 / TEXTURE_ATLAS_SIZE.height as f32,
            );
            let uv_rect = Rect::new(uv_pos, uv_size);

            vertices.extend_from_slice(&[
                Vertex {
                    position: tile_rect.min.to_array(),
                    uv: [uv_rect.min_x(), uv_rect.max_y()],
                    color: [1., 1., 1., 1.],
                },
                Vertex {
                    position: [tile_rect.max.x, tile_rect.min.y],
                    uv: [uv_rect.max_x(), uv_rect.max_y()],
                    color: [1., 1., 1., 1.],
                },
                Vertex {
                    position: [tile_rect.min.x, tile_rect.max.y],
                    uv: [uv_rect.min_x(), uv_rect.min_y()],
                    color: [1., 1., 1., 1.],
                },
                Vertex {
                    position: [tile_rect.max.x, tile_rect.min.y],
                    uv: [uv_rect.max_x(), uv_rect.max_y()],
                    color: [1., 1., 1., 1.],
                },
                Vertex {
                    position: tile_rect.max.to_array(),
                    uv: [uv_rect.max_x(), uv_rect.min_y()],
                    color: [1., 1., 1., 1.],
                },
                Vertex {
                    position: [tile_rect.min.x, tile_rect.max.y],
                    uv: [uv_rect.min_x(), uv_rect.min_y()],
                    color: [1., 1., 1., 1.],
                },
            ]);
        }
    }

    unsafe {
        let mut vertex_buffer = context.create_vertex_buffer().unwrap();
        vertex_buffer.write(&vertices);
        vertex_buffer
    }
}

// Time loops over 300 ticks, 5 seconds
const LOOP_TICKS: usize = 300;

const PLAYER_START: Point2D<f32> = Point2D {
    x: 320.,
    y: 180.,
    _unit: std::marker::PhantomData,
};

const GHOST_SPEED: f32 = 50.;

const GHOST_ANIMATION_FRAMES: u32 = 6;
const GHOST_ANIMATION_TIME: f32 = 0.5;

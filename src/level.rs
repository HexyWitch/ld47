use std::collections::HashMap;

use euclid::{
    default::{Box2D, Point2D, Rect},
    point2, size2,
};

use crate::{
    gl,
    graphics::{Vertex, TEXTURE_ATLAS_SIZE},
    texture_atlas::TextureRect,
};

pub const TILE_SIZE: u32 = 16;

const LEVEL_HEIGHT: usize = 22;
const LEVEL: [&'static str; LEVEL_HEIGHT] = [
    "########################################",
    "#    ##B                               #",
    "#    ##                                #",
    "#B    |      S                         #",
    "#######-#######                        #",
    "####### #######                        #",
    "#######      ##                        #",
    "#######      ##                        #",
    "###############                        #",
    "###############                        #",
    "#                                      #",
    "#                                      #",
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

pub fn create_level() -> Level {
    let mut tiles = Vec::new();
    let mut player_start = point2(0., 0.);
    let mut buttons = HashMap::new();
    let mut doors = HashMap::new();

    for y_tile in 0..LEVEL_HEIGHT {
        let mut row = Vec::new();
        for (x_tile, c) in LEVEL[LEVEL_HEIGHT - 1 - y_tile].chars().enumerate() {
            row.push(match c {
                ' ' => Tile::Floor,
                '#' => Tile::Wall,
                'S' => {
                    player_start = point2(x_tile as f32 + 0.5, y_tile as f32 + 0.5);
                    Tile::Floor
                }
                'B' => {
                    log::info!("Button at {:?}", (x_tile, y_tile));
                    buttons.insert(point2(x_tile as i32, y_tile as i32), ButtonTile::default());
                    Tile::Floor
                }
                '|' => {
                    doors.insert(point2(x_tile as i32, y_tile as i32), DoorTile::Vertical);
                    Tile::Floor
                }
                '-' => {
                    doors.insert(point2(x_tile as i32, y_tile as i32), DoorTile::Horizontal);
                    Tile::Floor
                }
                c => panic!("unknown tile type {}", c),
            })
        }
        tiles.push(row);
    }

    buttons
        .get_mut(&point2(7, 20))
        .expect("button not found")
        .connection = Some(point2(6, 18));

    buttons
        .get_mut(&point2(1, 18))
        .expect("button not found")
        .connection = Some(point2(7, 17));

    Level {
        tiles,
        player_start,
        buttons,
        doors,
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tile {
    Floor,
    Wall,
}

#[derive(Default)]
pub struct ButtonTile {
    pub connection: Option<Point2D<i32>>,
}

pub enum DoorTile {
    Horizontal,
    Vertical,
}

pub struct Level {
    pub tiles: Vec<Vec<Tile>>,
    pub player_start: Point2D<f32>,
    pub buttons: HashMap<Point2D<i32>, ButtonTile>,
    pub doors: HashMap<Point2D<i32>, DoorTile>,
}

impl Level {
    pub fn tile(&self, x: i32, y: i32) -> Tile {
        if x > 0 && y > 0 {
            *self
                .tiles
                .get(y as usize)
                .and_then(|row| row.get(x as usize))
                .unwrap_or(&Tile::Wall)
        } else {
            Tile::Wall
        }
    }

    fn height(&self) -> usize {
        self.tiles.len()
    }

    fn width(&self, y: usize) -> usize {
        self.tiles.get(y).map(|row| row.len()).unwrap_or(0)
    }
}

pub fn generate_tile_buffer(
    level: &Level,
    floor: TextureRect,
    walls: TextureRect,
    context: &mut gl::Context,
) -> gl::VertexBuffer {
    let mut vertices = Vec::new();

    for y_tile in 0..level.height() {
        for x_tile in 0..level.width(y_tile) {
            let tile = match level.tile(x_tile as i32, y_tile as i32) {
                Tile::Floor => floor,
                Tile::Wall => {
                    let tl = level.tile(x_tile as i32 - 1, y_tile as i32 + 1) == Tile::Wall;
                    let t = level.tile(x_tile as i32, y_tile as i32 + 1) == Tile::Wall;
                    let tr = level.tile(x_tile as i32 + 1, y_tile as i32 + 1) == Tile::Wall;
                    let l = level.tile(x_tile as i32 - 1, y_tile as i32) == Tile::Wall;
                    let r = level.tile(x_tile as i32 + 1, y_tile as i32) == Tile::Wall;
                    let bl = level.tile(x_tile as i32 - 1, y_tile as i32 - 1) == Tile::Wall;
                    let b = level.tile(x_tile as i32, y_tile as i32 - 1) == Tile::Wall;
                    let br = level.tile(x_tile as i32 + 1, y_tile as i32 - 1) == Tile::Wall;

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
            };

            let tile_rect = Box2D::new(
                point2(x_tile as f32, y_tile as f32),
                point2((x_tile + 1) as f32, (y_tile + 1) as f32),
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

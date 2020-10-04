use std::collections::{HashMap, HashSet};

use euclid::{
    default::{Box2D, Point2D, Rect},
    point2, size2,
};

use crate::{
    gl,
    graphics::{Vertex, TEXTURE_ATLAS_SIZE},
    texture_atlas::TextureRect,
};

pub struct Level {
    pub tiles: Vec<Vec<Tile>>,
    pub player_start: Point2D<f32>,
    pub buttons: HashMap<Point2D<i32>, ButtonTile>,
    pub doors: HashMap<Point2D<i32>, DoorTile>,
    pub teleporters: HashMap<Point2D<i32>, TeleporterTile>,
    pub bulbs: HashSet<Point2D<i32>>,
    pub the_machine: Point2D<i32>,
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

pub const TILE_SIZE: u32 = 16;

const LEVEL_HEIGHT: usize = 22;
const LEVEL: [&'static str; LEVEL_HEIGHT] = [
    "########################################",
    "########T  B##T####T########T###########",
    "########    ##O#### ######## ###########",
    "########T  B##T#### ##B  B## ###########",
    "## B##### #########O##B   ## ###########",
    "##  #####-#########-####  ## ###########",
    "#O T##T               ##  ## ###########",
    "######                |  B##T###########",
    "######               B##################",
    "## B##  S    M        ##################",
    "##  ##               T##T   T###########",
    "##O  |       T        #### #############",
    "### ##B               #### #############",
    "### ######-#####-#########  B T##T   B##",
    "### ###### #####-######### ########## ##",
    "###  B####  B###OB########  T#T B T##T##",
    "##########   ###  ######################",
    "#######T##T B###T#######################",
    "####### ####################TO B  T##T##",
    "#######T##T     T#################### ##",
    "#################################B    ##",
    "########################################",
];

const BUTTON_CONNECTIONS: [&'static str; LEVEL_HEIGHT] = [
    "########################################",
    "########D  D##D####X########X###########",
    "########    ## #### ######## ###########",
    "########E  E##E#### ##X  L## ###########",
    "## C##### ######### ##Z   ## ###########",
    "##  #####C#########Z####  ## ###########",
    "#  B##B               ##  ## ###########",
    "######                J  K##L###########",
    "######               J##################",
    "## B##                ##################",
    "##  ##               K##K   L###########",
    "##   A                #### #############",
    "### ##A               #### #############",
    "### ######F#####I#########  M N##N   O##",
    "### ###### #####H######### ########## ##",
    "###  F####  H### I########  M#M N O##P##",
    "##########   ###  ######################",
    "#######G##G G###G#######################",
    "####### ####################Q  P  O##P##",
    "#######H##H     G#################### ##",
    "#################################Q    ##",
    "########################################",
];

const TELEPORTER_CONNECTIONS: [&'static str; LEVEL_HEIGHT] = [
    "########################################",
    "########B   ##B####I########I###########",
    "########    ## #### ######## ###########",
    "########C   ##C#### ##    ## ###########",
    "##  ##### ######### ##    ## ###########",
    "##  ##### ######### ####  ## ###########",
    "#  A##A               ##  ## ###########",
    "######                    ##H###########",
    "######                ##################",
    "##  ##                ##################",
    "##  ##               G##G   H###########",
    "##           N        #### #############",
    "### ##                #### #############",
    "### ###### ##### #########    L##L    ##",
    "### ###### ##### ######### ########## ##",
    "###   ####   ###  ########  J#J   K##M##",
    "##########   ###  ######################",
    "#######D##D  ###F#######################",
    "####### ####################N     K##M##",
    "#######E##E     F#################### ##",
    "#################################     ##",
    "########################################",
];

pub fn create_level() -> Level {
    let mut tiles = Vec::new();
    let mut player_start = point2(0., 0.);
    let mut buttons = HashMap::new();
    let mut doors = HashMap::new();
    let mut teleporters = HashMap::new();
    let mut bulbs = HashSet::new();
    let mut the_machine = None;

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
                'T' => {
                    teleporters.insert(
                        point2(x_tile as i32, y_tile as i32),
                        TeleporterTile::default(),
                    );
                    Tile::Floor
                }
                'O' => {
                    bulbs.insert(point2(x_tile as i32, y_tile as i32));
                    Tile::Floor
                }
                'M' => {
                    the_machine = Some(point2(x_tile as i32, y_tile as i32));
                    Tile::Floor
                }
                c => panic!("unknown tile type {}", c),
            })
        }
        tiles.push(row);
    }

    let mut button_connections = HashMap::new();
    for y_tile in 0..LEVEL_HEIGHT {
        for (x_tile, c) in BUTTON_CONNECTIONS[LEVEL_HEIGHT - 1 - y_tile]
            .chars()
            .enumerate()
        {
            match c {
                '#' | ' ' => {}
                c => {
                    button_connections
                        .entry(c)
                        .or_insert_with(|| Vec::new())
                        .push(point2(x_tile as i32, y_tile as i32));
                }
            }
        }
    }

    for connections in button_connections.values() {
        if let Some(button_index) = connections.iter().enumerate().find_map(|(index, point)| {
            if buttons.get(point).is_some() {
                Some(index)
            } else {
                None
            }
        }) {
            let button = buttons.get_mut(&connections[button_index]).unwrap();
            for (i, point) in connections.iter().enumerate() {
                if i != button_index {
                    button.connections.push(*point);
                }
            }
        }
    }

    let mut teleporter_connections = HashMap::new();
    for y_tile in 0..LEVEL_HEIGHT {
        for (x_tile, c) in TELEPORTER_CONNECTIONS[LEVEL_HEIGHT - 1 - y_tile]
            .chars()
            .enumerate()
        {
            match c {
                '#' | ' ' => {}
                c => {
                    teleporter_connections
                        .entry(c)
                        .or_insert_with(|| Vec::new())
                        .push(point2(x_tile as i32, y_tile as i32));
                }
            }
        }
    }
    for connections in teleporter_connections.values() {
        if connections.len() == 2 {
            teleporters
                .get_mut(&connections[0])
                .expect("no teleporter found at connection point")
                .connection = Some(connections[1]);
            teleporters
                .get_mut(&connections[1])
                .expect("no teleporter found at connection point")
                .connection = Some(connections[0]);
        } else {
            panic!("Teleporter is connected in more than 2 places")
        }
    }

    Level {
        tiles,
        player_start,
        buttons,
        doors,
        teleporters,
        bulbs,
        the_machine: the_machine.expect("No TheMachine found"),
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tile {
    Floor,
    Wall,
}

#[derive(Default)]
pub struct ButtonTile {
    pub connections: Vec<Point2D<i32>>,
}

pub enum DoorTile {
    Horizontal,
    Vertical,
}

#[derive(Default)]
pub struct TeleporterTile {
    pub connection: Option<Point2D<i32>>,
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

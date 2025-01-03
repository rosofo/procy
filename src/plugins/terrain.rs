use rand::thread_rng;

use crate::prelude::*;

pub fn terrain_plugin(app: &mut App) {
    app.insert_resource(MapConfig {
        floor_idx: 35,
        wall_idx: 72,
        size: TilemapSize { x: 256, y: 256 },
        tile_size: TilemapTileSize { x: 12.0, y: 12.0 },
        grid_size: TilemapGridSize { x: 12.0, y: 12.0 },
    });
    app.add_event::<SetTiles>();
    app.init_resource::<Tileset>();
    app.add_systems(Startup, setup);
    app.add_systems(Update, (set_tile_textures, set_tilemap_collider));
}

pub const FLOOR: u32 = 35;
pub const WALL: u32 = 72;

#[derive(Resource)]
pub struct MapConfig {
    pub floor_idx: u32,
    pub wall_idx: u32,
    pub size: TilemapSize,
    pub tile_size: TilemapTileSize,
    pub grid_size: TilemapGridSize,
}

impl MapConfig {
    pub fn world_to_tile(&self, pos: Vec2) -> Option<TilePos> {
        let pos = pos
            + Vec2::new(
                self.size.x as f32 * self.tile_size.x,
                self.size.y as f32 * self.tile_size.y,
            ) / 2.
            + Vec2::new(self.tile_size.x, self.tile_size.y) / 2.;
        TilePos::from_world_pos(&pos, &self.size, &self.grid_size, &TilemapType::Square)
    }
    pub fn tile_to_world(&self, pos: TilePos) -> Vec2 {
        let pos = pos.center_in_world(&self.grid_size, &TilemapType::Square);
        pos - Vec2::new(
            self.size.x as f32 * self.tile_size.x,
            self.size.y as f32 * self.tile_size.y,
        ) / 2.
            - Vec2::new(self.tile_size.x, self.tile_size.y) / 2.
    }
}

fn setup(mut commands: Commands, tileset: Res<Tileset>, config: Res<MapConfig>) {
    let texture_handle = tileset.0.clone();
    let map_size = TilemapSize { x: 256, y: 256 };

    // Create a tilemap entity a little early.
    // We want this entity early because we need to tell each tile which tilemap entity
    // it is associated with. This is done with the TilemapId component on each tile.
    // Eventually, we will insert the `TilemapBundle` bundle on the entity, which
    // will contain various necessary components, such as `TileStorage`.
    let tilemap_entity = commands.spawn_empty().id();

    // To begin creating the map we will need a `TileStorage` component.
    // This component is a grid of tile entities and is used to help keep track of individual
    // tiles in the world. If you have multiple layers of tiles you would have a tilemap entity
    // per layer, each with their own `TileStorage` component.
    let mut tile_storage = TileStorage::empty(map_size);

    // Spawn the elements of the tilemap.
    // Alternatively, you can use helpers::filling::fill_tilemap.
    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(WALL),
                        ..Default::default()
                    },
                    TileType::Wall,
                ))
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 12.0, y: 12.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });
}

#[derive(Resource)]
pub struct Tileset(pub Handle<Image>);

impl FromWorld for Tileset {
    fn from_world(world: &mut World) -> Self {
        let tileset = world.load_asset("tiles/TileSet.png");
        Self(tileset)
    }
}

#[derive(Event)]
pub struct SetTiles(pub Vec<(TilePos, TileType)>);

#[derive(Component)]
pub enum TileType {
    Wall,
    Floor,
}

fn set_tile_textures(
    mut events: EventReader<SetTiles>,
    tile_storage: Single<&TileStorage>,
    mut tiles: Query<(Entity, &mut TileTextureIndex, &mut TileType)>,
    config: Res<MapConfig>,
) {
    for event in events.read() {
        let entities = event
            .0
            .iter()
            .map(|(pos, _)| tile_storage.get(pos).unwrap());
        let mut indices = tiles.iter_many_mut(entities);
        let mut i = 0;
        while let Some((entity, mut idx, mut tile_type)) = indices.fetch_next() {
            let tile = &event.0[i].1;
            i += 1;
            match tile {
                TileType::Floor => {
                    idx.0 = config.floor_idx;
                    *tile_type = TileType::Floor;
                }
                TileType::Wall => {
                    idx.0 = config.wall_idx;
                    *tile_type = TileType::Wall;
                }
            }
        }
    }
}

fn set_tilemap_collider(
    mut events: EventReader<SetTiles>,
    tile_storage: Single<(Entity, &TileStorage)>,
    mut commands: Commands,
) {
    for SetTiles(tiles) in events.read() {
        let tile_collider = Collider::cuboid(6.0, 6.0);
        let collider = Collider::compound(
            tiles
                .iter()
                .filter_map(|(pos, tile)| {
                    if let TileType::Floor = tile {
                        return None;
                    }
                    let translation = Vect::new(pos.x as f32 * 12.0, pos.y as f32 * 12.0);
                    Some((translation, Rot::default(), tile_collider.clone())) // cheap clone (internal Arc)
                })
                .collect_vec(),
        );
        commands
            .entity(tile_storage.0)
            .insert((collider, RigidBody::Fixed));
    }
}

fn debug(
    mut cursor: Local<Vec2>,
    mut events: EventReader<CursorMoved>,
    config: Res<MapConfig>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    if let Some(e) = events.read().last() {
        *cursor = e.position;
    }

    let world = camera.0.viewport_to_world_2d(camera.1, *cursor).unwrap();
    let tile = config.world_to_tile(world);
    let back_to_world = tile.map(|pos| config.tile_to_world(pos));

    debug!(
        "cursor: {:?}, world: {:?}, world_to_tile: {:?}, tile_to_world: {:?}",
        cursor, world, tile, back_to_world
    );
}

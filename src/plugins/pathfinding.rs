use bevy::color::ColorCurve;
use ndarray::{Array2, Axis};

use crate::prelude::*;

use super::terrain::TileType;

pub fn pathfinding_plugin(app: &mut App) {
    app.add_systems(Update, (update_dmap).run_if(on_event::<UpdateDMap>));
    app.add_event::<UpdateDMap>();
}

#[derive(Component)]
pub struct DMap {
    values: Array2<Option<u32>>,
    tile_storage: Entity,
}
#[derive(Component)]
pub struct Goal;

impl DMap {
    pub fn new(width: usize, height: usize, tile_storage: Entity) -> Self {
        Self {
            values: Array2::from_elem((width, height), None),
            tile_storage,
        }
    }
    pub fn get(&self, pos: IVec2) -> Option<u32> {
        self.values
            .get((pos.x as usize, pos.y as usize))
            .copied()
            .flatten()
    }
    fn reset(&mut self) {
        self.values.fill(None);
    }
    fn generate(&mut self) {
        let mut dirty = true;
        let max_iter = 50;
        let mut n = 0;
        while dirty && n < max_iter {
            dirty = false;
            // cell_view gives interior mutability, and it seems this algo calls for mutating as we go
            // https://www.roguebasin.com/index.php/The_Incredible_Power_of_Dijkstra_Maps
            let cells = self.values.cell_view();
            for (x, row) in cells.axis_iter(Axis(0)).enumerate() {
                for (y, cell) in row.iter().enumerate() {
                    if cell.get().is_none() {
                        continue;
                    }
                    let left = (x.saturating_sub(1), y);
                    let right = ((x + 1).clamp(0, 255), y);
                    let up = (x, y.saturating_sub(1));
                    let down = (x, (y + 1).clamp(0, 255));
                    let min = [(x, y), left, right, up, down]
                        .iter()
                        .filter_map(|idx| cells.get(*idx))
                        .filter_map(|c| c.get())
                        .min();
                    if let Some(min) = min {
                        if min < cell.get().unwrap_or_default() {
                            cell.set(Some(min.saturating_add(1)));
                            dirty = true;
                        }
                    }
                }
            }

            n += 1;
        }
        debug!("updated dmap: {:?}", self.values);
        if n >= max_iter {
            warn!("reached max iterations while updating dmap");
        }
    }
}

#[derive(Event)]
pub struct UpdateDMap(pub Entity);

fn update_dmap(
    mut events: EventReader<UpdateDMap>,
    mut dmaps: Query<&mut DMap>,
    goals: Query<(), With<Goal>>,
    tile_storages: Query<&TileStorage>,
    tiles: Query<(&TilePos, &TileType, &TileColor)>,
) {
    if goals.is_empty() {
        return;
    }
    for UpdateDMap(entity) in events.read() {
        debug!("update dmap");
        let Ok(mut dmap) = dmaps.get_mut(*entity) else {
            warn!("non-existent dmap");
            continue;
        };
        dmap.reset();

        let Ok(tile_storage) = tile_storages.get(dmap.tile_storage) else {
            continue;
        };

        for tile in tile_storage.iter() {
            let tile = tile.unwrap();
            let (pos, tile_type, _color) = tiles.get(tile).unwrap();
            if goals.contains(tile) {
                dmap.values[(pos.x as usize, pos.y as usize)] = Some(0);
            } else if let TileType::Floor = tile_type {
                dmap.values[(pos.x as usize, pos.y as usize)] = Some(u32::MAX);
            }
        }

        dmap.generate();
    }
}

fn debug_render(
    dmap: Single<&DMap>,
    tile_storage: Single<&TileStorage>,
    mut tiles: Query<(&TilePos, &TileType, &mut TileColor)>,
    mut commands: Commands,
    tile_labels: Query<Entity, With<TileLabel>>,
) {
    tile_labels
        .iter()
        .for_each(|label| commands.entity(label).despawn());

    let palette = ColorCurve::new([RED, PINK, SKY_BLUE, LIGHT_BLUE]).unwrap();
    for tile in tile_storage.iter() {
        let tile = tile.unwrap();
        let (pos, _, mut color) = tiles.get_mut(tile).unwrap();
        let val = dmap.values[(pos.x as usize, pos.y as usize)];
        if let Some(val) = val {
            if val == u32::MAX {
                color.0 = ORANGE.into();
            } else {
                color.0 = palette
                    .sample(val.clamp(0, 30) as f32 / 30.0)
                    .unwrap()
                    .into();
            }
            if val <= 10 {
                commands.spawn((
                    TileLabel(tile),
                    Text2d(val.to_string()),
                    TextFont::from_font_size(8.0),
                    Transform::from_xyz(
                        (pos.x as i32 - 128) as f32 * 12.0,
                        (pos.y as i32 - 128) as f32 * 12.0,
                        1.0,
                    ),
                ));
            }
        } else {
            color.0 = Color::default();
        }
    }
}

#[derive(Component)]
pub struct TileLabel(Entity);

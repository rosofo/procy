use ndarray::{Array2, Axis};

use crate::prelude::*;

use super::terrain::TileType;

pub fn pathfinding_plugin(app: &mut App) {
    app.add_systems(Update, update_dmap.run_if(on_event::<UpdateDMap>));
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
    fn reset(&mut self) {
        self.values.fill(None);
    }
    fn generate(&mut self) {
        let mut dirty = true;
        let max_iter = 20;
        let mut n = 0;
        while dirty && n < max_iter {
            dirty = false;
            // cell_view gives interior mutability, and it seems this algo calls for mutating as we go
            // https://www.roguebasin.com/index.php/The_Incredible_Power_of_Dijkstra_Maps
            let cells = self.values.cell_view();
            for window in cells.windows((3, 3)) {
                // skip walls
                let Some(val) = window[(1, 1)].get() else {
                    continue;
                };
                // skip goals
                if val == 0 {
                    continue;
                }

                let min = window
                    .iter()
                    .enumerate()
                    .filter_map(|(i, x)| if i == 4 { None } else { x.get() })
                    .min();
                if let Some(min) = min {
                    window[(1, 1)].set(Some(min.saturating_add(1)));
                    // marking the fact that we mutated so that the outer while loop will try again
                    dirty = true;
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
    tiles: Query<(&TilePos, &TileType)>,
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
            let (pos, tile_type) = tiles.get(tile).unwrap();
            if goals.contains(tile) {
                dmap.values[(pos.x as usize, pos.y as usize)] = Some(0);
            } else if let TileType::Floor = tile_type {
                dmap.values[(pos.x as usize, pos.y as usize)] = Some(u32::MAX);
            }
        }

        dmap.generate();
    }
}

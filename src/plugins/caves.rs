use std::time::Duration;

use crate::prelude::*;
use bevy::{
    asset::RenderAssetUsages, color::ColorCurve, core_pipeline::deferred::node,
    utils::tracing::instrument,
};
use bevy_vector_shapes::painter::CanvasBundle;
use flat_spatial::Grid;
use image::{GrayImage, Luma};
use ndarray::{parallel::prelude::IntoParallelRefIterator, Array2, ShapeBuilder};
use ops::FloatPow;
use petgraph::{
    prelude::*,
    visit::{IntoEdges, IntoNodeReferences},
};
use rand::{thread_rng, Rng};

pub fn caves_plugin(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            seed,
            connect,
            ((populate_tiles, show_tiles).chain()),
            finish,
            tick,
        )
            .chain(),
    );
}

pub struct CaveNode {
    pub position: Vec2,
    pub radius: f32,
}
pub struct CaveEdge {
    pub width: f32,
}

#[derive(Component, Default)]
#[require(Transform(|| Transform::from_scale(Vec3::splat(2.0)).with_translation(Vec3::new(-300.0,-300.0,0.0))), Generating, InheritedVisibility)]
pub struct Caves {
    pub size: Vec2,
    pub graph: UnGraph<CaveNode, CaveEdge>,
}

#[derive(Component, Default)]
pub struct Generating;

#[derive(Clone)]
pub enum Tile {
    Wall,
    Floor,
}

#[derive(Component)]
pub struct CaveMap {
    pub map: Array2<Tile>,
}

#[instrument(skip(caves, cmd, shapes))]
fn show_graph(
    caves: Query<(Entity, &Caves), With<Generating>>,
    mut cmd: Commands,
    shapes: ShapeCommands,
) {
    let curve = ColorCurve::new([RED, GREEN, BLUE]).unwrap();
    for (entity, caves) in caves.iter() {
        if let Some(mut entity) = cmd.get_entity(entity) {
            entity.with_shape_children(shapes.config(), |parent| {
                debug!("draw edges");
                for edge in caves.graph.edge_indices() {
                    let (a, b) = caves.graph.edge_endpoints(edge).unwrap();
                    let a = &caves.graph[a];
                    let b = &caves.graph[b];
                    let dist = a.position.distance(b.position);
                    parent.set_color(curve.sample_clamped(dist / 256.0));
                    parent.line(a.position.extend(0.0), b.position.extend(0.0));
                }
                debug!("draw nodes");
                for node in caves.graph.node_indices() {
                    let node = &caves.graph[node];
                    parent.set_translation(node.position.extend(0.0));
                    parent.set_color(curve.sample_clamped(node.radius / 256.0));
                    parent.circle(node.radius / 10.0);
                }
            });
        }
    }
}

fn seed(mut caves: Query<&mut Caves, With<Generating>>) {
    for mut system in caves.iter_mut() {
        random_bsp(system.size).into_iter().for_each(|node| {
            system.graph.add_node(node);
        });
    }
}

#[instrument(skip(caves))]
fn connect(mut caves: Query<&mut Caves, With<Generating>>) {
    let mut rng = thread_rng();
    for mut system in caves.iter_mut() {
        debug!("fill spatial grid");
        let mut g: Grid<NodeIndex, [f32; 2]> = Grid::new(32);
        for (node, weight) in system.graph.node_references() {
            g.insert([weight.position.x, weight.position.y], node);
        }

        debug!("add edges");
        for node in system.graph.node_indices() {
            let weight = &system.graph[node];
            let neighbors =
                g.query_around([weight.position.x, weight.position.y], weight.radius / 2.0);
            for (handle, _pos) in neighbors.take((64.0 - weight.radius) as usize + 1) {
                if rng.gen_bool(0.5) {
                    let (_, id) = g.get(handle).unwrap();
                    system.graph.add_edge(node, *id, CaveEdge { width: 1.0 });
                }
            }
        }
    }
}

fn finish(caves: Query<Entity, With<Generating>>, mut commands: Commands) {
    for entity in caves.iter() {
        commands.entity(entity).remove::<Generating>();
    }
}

#[instrument]
fn random_bsp(size: Vec2) -> Vec<CaveNode> {
    let mut rng = thread_rng();
    let mut nodes = vec![];

    let min_area = 64.0;

    let mut stack = vec![Rect::new(0.0, 0.0, size.x, size.y)];

    let mut push_node = |rect: Rect| {
        debug!("leaf");
        nodes.push(CaveNode {
            position: rect.center(),
            radius: rect.width().max(rect.height()),
        });
    };

    let mut rng_ = rng.clone();
    let mut split_rect = |stack: &mut Vec<Rect>, rect: Rect| {
        debug!("split");
        if rng_.gen_bool(0.5) {
            let left = Rect::new(
                rect.min.x,
                rect.min.y,
                rect.min.x + rect.width() / 2.0,
                rect.max.y,
            );
            let right = Rect::new(
                rect.min.x + rect.width() / 2.0,
                rect.min.y,
                rect.max.x,
                rect.max.y,
            );
            stack.push(left);
            stack.push(right);
        } else {
            let top = Rect::new(
                rect.min.x,
                rect.min.y,
                rect.max.x,
                rect.min.y + rect.height() / 2.0,
            );
            let bottom = Rect::new(
                rect.min.x,
                rect.min.y + rect.height() / 2.0,
                rect.max.x,
                rect.max.y,
            );
            stack.push(top);
            stack.push(bottom);
        }
    };

    while let Some(rect) = stack.pop() {
        let area = rect.size().element_product();
        let chance = (size.element_product() - area).remap(0.0, size.element_product(), 0.0, 1.0);
        if rect.size().element_product() < min_area || rng.gen_bool((chance as f64 * 0.1).powi(2)) {
            push_node(rect);
        } else {
            split_rect(&mut stack, rect);
        }
    }

    nodes
}

fn tick(
    mut timer: Local<Timer>,
    time: Res<Time>,
    caves: Query<Entity, With<Caves>>,
    mut commands: Commands,
) {
    if timer.tick(time.delta()).just_finished() {
        for entity in caves.iter() {
            if let Some(e) = commands.get_entity(entity) {
                e.try_despawn_recursive()
            }
        }

        commands.spawn(Caves {
            size: Vec2::new(256.0, 256.0),
            ..default()
        });

        timer.set_duration(Duration::from_secs(1));
        timer.reset();
    }
}

#[instrument(skip(maps, cmd, shapes))]
fn show_tiles(
    maps: Query<(Entity, &CaveMap), With<Generating>>,
    mut cmd: Commands,
    shapes: ShapeCommands,
) {
    debug!("show tiles");
    let scale = 1.0;
    for (entity, map) in maps.iter() {
        cmd.entity(entity)
            .with_shape_children(shapes.config(), |parent| {
                for ((x, y), tile) in map.map.indexed_iter() {
                    if let Tile::Wall = *tile {
                        parent.set_translation(
                            Vec2::new(x as f32 * scale, y as f32 * scale).extend(1.0),
                        );
                        parent.circle(1.0);
                    }
                }
            });
    }
}

#[instrument(skip(caves, commands))]
fn populate_tiles(
    caves: Query<(Entity, &Caves), (Without<CaveMap>, With<Generating>)>,
    mut commands: Commands,
) {
    debug!("populate tiles");
    for (entity, system) in caves.iter() {
        let mut img = image::GrayImage::new(256, 256);
        for node in system.graph.node_weights() {
            imageproc::drawing::draw_filled_circle_mut(
                &mut img,
                (node.position.x as i32, node.position.y as i32),
                (node.radius.ceil() * 0.1) as i32,
                Luma([255]),
            );
        }
        for edge in system.graph.edge_references() {
            tunnel_between(&system.graph, edge.source(), edge.target(), &mut img);
        }
        let map = img
            .rows()
            .flat_map(|row| {
                row.map(|pixel| {
                    if pixel.0[0] == 0 {
                        Tile::Wall
                    } else {
                        Tile::Floor
                    }
                })
            })
            .collect_vec();
        let map = Array2::from_shape_vec((256, 256).strides((1, 256)), map).unwrap();
        commands.entity(entity).try_insert(CaveMap { map });
    }
}

fn tunnel_between(
    graph: &UnGraph<CaveNode, CaveEdge>,
    source: NodeIndex,
    target: NodeIndex,
    map: &mut GrayImage,
) {
    let mut rng = thread_rng();
    let a = graph.node_weight(source).unwrap();
    let b = graph.node_weight(target).unwrap();
    let dir = b.position - a.position;
    for i in 0..10 {
        let t = (i as f32 / 10.0).squared();
        let dir = Rot2::degrees(rng.gen_range(-45.0..45.0)) * dir;
        imageproc::drawing::draw_filled_circle_mut(
            map,
            (a.position + dir * t).as_ivec2().into(),
            (b.radius * 0.1 * t + a.radius * 0.1 * (1.0 - t)) as i32,
            Luma([255]),
        );
    }
}

use std::time::Duration;

use crate::{
    math::trunc_falloff,
    plugins::terrain::{TileType, FLOOR, WALL},
    prelude::*,
};
use bevy::{
    color::ColorCurve, input::common_conditions::input_just_pressed, utils::tracing::instrument,
};
use flat_spatial::Grid;
use image::{GrayImage, Luma};
use ops::FloatPow;
use petgraph::{prelude::*, visit::IntoNodeReferences};
use rand::{thread_rng, Rng};

use super::{
    pathfinding::{DMap, UpdateDMap},
    terrain::SetTiles,
};

pub fn caves_plugin(app: &mut App) {
    app.insert_resource(Config {
        min_area: 16.0,
        grid_size: 64,
        edge_neighbors: 3,
        tunnel_segments: 10,
        tunnel_angle: 45.0,
        tunnel_thickness: 0.1,
        node_radius_factor: 0.1,
        node_color_factor: 256.0,
        edge_color_factor: 256.0,
        trunc_falloff_factor: 0.05,
    });
    app.add_event::<Regen>();
    app.add_systems(Update, ui);
    app.add_systems(
        Update,
        regen.run_if(input_just_pressed(KeyCode::Space).or(on_event::<Regen>)),
    );
    app.add_systems(
        FixedUpdate,
        ((seed, connect, populate_tiles, finish).chain(), insert_dmap),
    );
}

fn ui(mut contexts: EguiContexts, mut config: ResMut<Config>, mut events: EventWriter<Regen>) {
    egui::Window::new("Caves").show(contexts.ctx_mut(), |ui| {
        let mut regen = false;
        regen |= ui
            .add(egui::Slider::new(&mut config.min_area, 1.0..=255.0).text("min node area"))
            .drag_stopped();
        regen |= ui
            .add(egui::Slider::new(&mut config.grid_size, 1..=128).text("grid size"))
            .drag_stopped();
        regen |= ui
            .add(egui::Slider::new(&mut config.edge_neighbors, 1..=10).text("edge neighbors"))
            .drag_stopped();
        regen |= ui
            .add(egui::Slider::new(&mut config.tunnel_segments, 1..=20).text("tunnel segments"))
            .drag_stopped();
        regen |= ui
            .add(egui::Slider::new(&mut config.tunnel_angle, 0.0..=90.0).text("tunnel angle"))
            .drag_stopped();
        regen |= ui
            .add(
                egui::Slider::new(&mut config.tunnel_thickness, 0.01..=1.0)
                    .text("tunnel thickness"),
            )
            .drag_stopped();
        regen |= ui
            .add(
                egui::Slider::new(&mut config.node_radius_factor, 0.01..=1.0)
                    .text("node radius factor"),
            )
            .drag_stopped();
        regen |= ui
            .add(
                egui::Slider::new(&mut config.node_color_factor, 1.0..=512.0)
                    .text("node color factor"),
            )
            .drag_stopped();
        regen |= ui
            .add(
                egui::Slider::new(&mut config.edge_color_factor, 1.0..=512.0)
                    .text("edge color factor"),
            )
            .drag_stopped();
        regen |= ui
            .add(
                egui::Slider::new(&mut config.trunc_falloff_factor, 0.01..=1.0)
                    .text("trunc falloff factor"),
            )
            .drag_stopped();

        if regen {
            events.send(Regen);
        }
    });
}

#[derive(Event)]
pub struct Regen;

#[derive(Resource)]
pub struct Config {
    min_area: f32,
    grid_size: usize,
    edge_neighbors: usize,
    tunnel_segments: usize,
    tunnel_angle: f32,
    tunnel_thickness: f32,
    node_radius_factor: f32,
    node_color_factor: f32,
    edge_color_factor: f32,
    trunc_falloff_factor: f32,
}

pub struct CaveNode {
    pub position: Vec2,
    pub radius: f32,
}
pub struct CaveEdge {
    pub width: f32,
}

#[derive(Component, Default)]
#[require(Transform, Generating, InheritedVisibility)]
pub struct Caves {
    pub size: Vec2,
    pub graph: UnGraph<CaveNode, CaveEdge>,
}

#[derive(Component, Default)]
pub struct Generating;

fn seed(mut caves: Query<&mut Caves, With<Generating>>, config: Res<Config>) {
    for mut system in caves.iter_mut() {
        random_bsp(system.size, &config)
            .into_iter()
            .for_each(|node| {
                system.graph.add_node(node);
            });
    }
}

#[instrument(skip(caves, config))]
fn connect(mut caves: Query<&mut Caves, With<Generating>>, config: Res<Config>) {
    for mut system in caves.iter_mut() {
        debug!("fill spatial grid");
        let mut g: Grid<NodeIndex, [f32; 2]> = Grid::new(config.grid_size as i32);
        for (node, weight) in system.graph.node_references() {
            g.insert([weight.position.x, weight.position.y], node);
        }

        debug!("add edges");
        for node in system.graph.node_indices() {
            let weight = &system.graph[node];
            let neighbors = g.query_around([weight.position.x, weight.position.y], weight.radius);
            for (handle, _pos) in neighbors.take(config.edge_neighbors) {
                let (_, id) = g.get(handle).unwrap();
                system.graph.add_edge(node, *id, CaveEdge { width: 1.0 });
            }
        }
    }
}

fn finish(caves: Query<Entity, With<Generating>>, mut commands: Commands) {
    for entity in caves.iter() {
        commands.entity(entity).remove::<Generating>();
    }
}

#[instrument(skip(config))]
fn random_bsp(size: Vec2, config: &Config) -> Vec<CaveNode> {
    let mut rng = thread_rng();
    let mut nodes = vec![];

    let mut stack = vec![Rect::new(0.0, 0.0, size.x, size.y)];

    let mut push_node = |rect: Rect| {
        nodes.push(CaveNode {
            position: rect.center(),
            radius: rect.width().max(rect.height()),
        });
    };

    let mut rng_ = rng.clone();
    let mut split_rect = |stack: &mut Vec<Rect>, rect: Rect| {
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
        let chance = area.remap(config.min_area, size.element_product(), 0.0, 1.0);
        let chance = trunc_falloff(chance, 1.0) * config.trunc_falloff_factor;
        if rect.size().element_product() < config.min_area || rng.gen_bool(chance as f64) {
            push_node(rect);
        } else {
            split_rect(&mut stack, rect);
        }
    }

    nodes
}

fn regen(caves: Query<Entity, With<Caves>>, mut commands: Commands) {
    for entity in caves.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.try_despawn_recursive()
        }
    }

    commands.spawn(Caves {
        size: Vec2::new(256.0, 256.0),
        ..default()
    });
}

#[instrument(skip(caves, events, config))]
fn populate_tiles(
    caves: Query<(Entity, &Caves), With<Generating>>,
    mut events: EventWriter<SetTiles>,
    config: Res<Config>,
) {
    for (entity, system) in caves.iter() {
        debug!("populate tiles");
        let mut img = image::GrayImage::new(256, 256);
        for node in system.graph.node_weights() {
            imageproc::drawing::draw_filled_circle_mut(
                &mut img,
                (node.position.x as i32, node.position.y as i32),
                (node.radius.ceil() * config.node_radius_factor) as i32,
                Luma([255]),
            );
        }
        for edge in system.graph.edge_references() {
            tunnel_between(
                &system.graph,
                edge.source(),
                edge.target(),
                &mut img,
                &config,
            );
        }

        let set_tiles = img
            .iter()
            .enumerate()
            .map(|(i, pixel)| {
                let x = i as u32 % 256;
                let y = i as u32 / 256;
                (
                    TilePos { x, y },
                    if *pixel == 255 {
                        TileType::Floor
                    } else {
                        TileType::Wall
                    },
                )
            })
            .collect_vec();

        events.send(SetTiles(set_tiles));
    }
}

fn tunnel_between(
    graph: &UnGraph<CaveNode, CaveEdge>,
    source: NodeIndex,
    target: NodeIndex,
    map: &mut GrayImage,
    config: &Config,
) {
    let mut rng = thread_rng();
    let a = graph.node_weight(source).unwrap();
    let b = graph.node_weight(target).unwrap();
    let dir = b.position - a.position;
    for i in 0..config.tunnel_segments {
        let t = (i as f32 / config.tunnel_segments as f32).squared();
        let dir = Rot2::degrees(rng.gen_range(-config.tunnel_angle..config.tunnel_angle)) * dir;
        imageproc::drawing::draw_filled_circle_mut(
            map,
            (a.position + dir * t).as_ivec2().into(),
            (b.radius * config.tunnel_thickness * t
                + a.radius * config.tunnel_thickness * (1.0 - t)) as i32,
            Luma([255]),
        );
    }
}

fn insert_dmap(
    tile_storage: Single<Entity, With<TileStorage>>,
    mut commands: Commands,
    caves: Query<Entity, (With<Caves>, Without<Generating>, Without<DMap>)>,
) {
    for cave in caves.iter() {
        debug!("insert dmap for caves");
        let mut cave = commands.entity(cave);
        let dmap = cave.insert(DMap::new(256, 256, *tile_storage)).id();
        commands.send_event(UpdateDMap(dmap));
    }
}

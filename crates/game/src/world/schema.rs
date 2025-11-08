use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::systems::economy::{HubId, RouteId, Weather};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorldGraph {
    pub hubs: HashMap<String, HubSpec>,
    pub links: HashMap<String, LinkSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HubSpec {
    pub name: String,
    pub x_mm: i32,
    pub y_mm: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSpec {
    pub from: String,
    pub to: String,
    pub style: String,
    pub base_minutes: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Board {
    pub link_id: RouteId,
    pub style: String,
    pub weather: Weather,
    pub cell_mm: u32,
    pub dims: BoardDims,
    pub walls: Vec<Wall>,
    pub cover: Vec<Cover>,
    pub spawns: SpawnPoints,
    pub zones: Zones,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoardDims {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Wall {
    pub x: i32,
    pub y: i32,
    pub len: u32,
    pub dir: Direction,
}

#[derive(Debug, Clone, Serialize)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cover {
    pub x: i32,
    pub y: i32,
    pub kind: CoverKind,
}

#[derive(Debug, Clone, Serialize)]
pub enum CoverKind {
    Rock,
    Tree,
    Bush,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpawnPoints {
    pub enemy: Vec<Point>,
    pub player: Vec<Point>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Zones {
    pub hold: Vec<Rectangle>,
    pub evac: Vec<Rectangle>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}
use hai_macros::node;
use log::warn;
use std::sync::{Arc, Mutex};
use winit::dpi::LogicalSize;
use std::any::Any;

use crate::traits::{Node};
use crate::{
    sprite::Sprite,
    types::{Point, PointF, Transform},
};

static mut NODE_ID: u32 = 0;

#[node]
#[derive(Debug, Default)]
pub struct Container {}

impl Container {}

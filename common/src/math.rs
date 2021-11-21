use nalgebra::{Point2, Vector2, Vector3};
use std::fmt::Debug;

pub type Vec2u = Vector2<u32>;
pub type Vec2i = Vector2<i32>;
pub type Vec2 = Vector2<f64>;
pub type Vec2f = Vector2<f32>;
pub type Vec3u = Vector3<u32>;
pub type Vec3i = Vector3<i32>;
pub type Vec3 = Vector3<f64>;
pub type Vec3f = Vector3<f32>;
pub type Pt2u = Point2<u32>;
pub type Pt2i = Point2<i32>;
pub type Pt2 = Point2<f64>;
pub type Pt2f = Point2<f32>;

/// Workaround function to get around rust-analyzer not handling Point2::new()'s "overloads" correctly
pub fn pt2<T>(x: T, y: T) -> Point2<T>
where T: Clone + Debug + PartialEq + 'static
{
    Point2::from(Vector2::from([x, y]))
}
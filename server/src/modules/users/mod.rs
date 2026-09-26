//! Users: the user model and its data access. No HTTP routes of its own yet.

pub mod model;
pub mod repository;

pub use model::{PublicUser, User};

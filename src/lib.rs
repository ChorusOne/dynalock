//! Some docs go here

extern crate core;

#[macro_use]
extern crate maplit;

#[cfg(feature = "dynamodb")]
extern crate rusoto_core;
#[cfg(feature = "dynamodb")]
extern crate rusoto_dynamodb;
#[cfg(feature = "dynamodb")]
extern crate uuid;

pub mod error;
pub mod providers;

pub use providers::DistLock;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dist_lock_new() {
        assert!(true);
    }
}

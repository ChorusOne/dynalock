//! Some docs go here

extern crate core;

#[cfg(feature = "dynamodb")]
extern crate rusoto_core;
#[cfg(feature = "dynamodb")]
extern crate rusoto_dynamodb;

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

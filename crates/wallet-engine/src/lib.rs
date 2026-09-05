#![forbid(unsafe_code)]

/// Returns the fixed identity of the isolated wallet engine crate.
#[must_use]
pub const fn engine_name() -> &'static str {
    "scout-wallet-lab"
}

#[cfg(test)]
mod tests {
    use super::engine_name;

    #[test]
    fn engine_identity_is_stable() {
        assert_eq!(engine_name(), "scout-wallet-lab");
    }
}

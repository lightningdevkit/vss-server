pub(crate) mod config;
pub(crate) mod logger;

use api::types::KeyValue;

/// A copy of the same printer used in vss-client to keep logs consistent.
pub(crate) struct KeyValueVecKeyPrinter<'a>(pub(crate) &'a Vec<KeyValue>);

impl core::fmt::Display for KeyValueVecKeyPrinter<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "[")?;
		for (i, k) in self.0.iter().enumerate() {
			if i == self.0.len() - 1 {
				write!(f, "{}", &k.key)?;
			} else {
				write!(f, "{}, ", &k.key)?;
			}
		}
		write!(f, "]")?;
		Ok(())
	}
}

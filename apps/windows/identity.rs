// Shared by the app and dependency-free hook. Stable is set only by release stamping.
pub const STABLE: bool = cfg!(feature = "stable-channel");
pub const NAMESPACE: &str = if STABLE { "io.github.moha-b.notch" } else { "io.github.moha-b.notch.preview" };
pub const NAME: &str = if STABLE { "Notch" } else { "Notch Preview" };
pub const PORT: u16 = if STABLE { 48766 } else { 48767 };

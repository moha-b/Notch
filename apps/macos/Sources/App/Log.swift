import os

/// An agent app has no window to print into, so anything worth diagnosing has
/// to go somewhere you can read it:
///
///     log stream --predicate 'subsystem == "io.github.moha-b.notch"' --level debug
enum Log {
    static let usage = Logger(subsystem: "io.github.moha-b.notch", category: "usage")
    static let sessions = Logger(subsystem: "io.github.moha-b.notch", category: "sessions")
}

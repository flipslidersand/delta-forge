/// A single recomputation event.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub node: String,
    pub recomputed: bool,
    pub duration_us: u64,
}

/// Accumulated recomputation log.
#[derive(Debug, Default)]
pub struct Log {
    entries: Vec<LogEntry>,
}

impl Log {
    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }

    pub fn print(&self) {
        if self.entries.is_empty() {
            println!("(no recomputations logged)");
            return;
        }
        println!("{:<20} {:>12} {:>12}", "node", "recomputed", "μs");
        println!("{}", "-".repeat(46));
        for e in &self.entries {
            println!("{:<20} {:>12} {:>12}", e.node, e.recomputed, e.duration_us);
        }
    }

    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

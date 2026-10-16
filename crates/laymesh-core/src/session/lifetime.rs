//! Cache ownership follows the live evaluation graph, not a memory quota.
//! A replayed parent retains its children without visiting or copying them.
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum Key {
    Query(u64, u64),
    Node(String),
    Geometry(String),
    Path([u8; 32]),
    Text(String),
}

#[derive(Default)]
pub(crate) struct Lifetime {
    roots: HashMap<String, HashSet<Key>>,
    pending: HashMap<String, HashSet<Key>>,
    edges: HashMap<Key, HashSet<Key>>,
    stack: Vec<(Key, HashSet<Key>)>,
    reads: HashSet<Key>,
}

impl Lifetime {
    pub fn start(&mut self) {
        self.stack.clear();
        self.reads.clear();
    }
    pub fn touch(&mut self, key: Key) {
        if let Some((_, reads)) = self.stack.last_mut() {
            reads.insert(key);
        } else {
            self.reads.insert(key);
        }
    }
    pub fn begin(&mut self, key: Key, preserve: bool) {
        self.touch(key.clone());
        let reads = if preserve {
            self.edges.get(&key).cloned().unwrap_or_default()
        } else {
            HashSet::new()
        };
        self.stack.push((key, reads));
    }
    pub fn end(&mut self) {
        if let Some((key, reads)) = self.stack.pop() {
            self.edges.insert(key, reads);
        }
    }
    pub fn finish(&mut self, file: &str, success: bool) -> HashSet<Key> {
        while !self.stack.is_empty() {
            self.end();
        }
        let reads = std::mem::take(&mut self.reads);
        if success {
            self.roots.insert(file.into(), reads);
            self.pending.remove(file);
        } else {
            // Keep only the latest interrupted attempt alongside the last
            // successful scene. Completed queries remain useful on retry.
            self.pending.insert(file.into(), reads);
        }
        self.live()
    }
    pub fn remove_file(&mut self, file: &str) -> HashSet<Key> {
        self.roots.remove(file);
        self.pending.remove(file);
        self.live()
    }
    pub fn retain_files(&mut self, files: &HashSet<String>) -> HashSet<Key> {
        self.roots.retain(|file, _| files.contains(file));
        self.pending.retain(|file, _| files.contains(file));
        self.live()
    }
    fn live(&mut self) -> HashSet<Key> {
        let mut work: Vec<_> = self
            .roots
            .values()
            .chain(self.pending.values())
            .flatten()
            .cloned()
            .collect();
        let mut live = HashSet::new();
        while let Some(key) = work.pop() {
            if live.insert(key.clone()) {
                if let Some(children) = self.edges.get(&key) {
                    work.extend(children.iter().cloned());
                }
            }
        }
        self.edges.retain(|key, _| live.contains(key));
        live
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_keeps_children_and_replacing_or_removing_a_page_releases_them() {
        let mut graph = Lifetime::default();
        for page in 0..6 {
            graph.start();
            graph.begin(Key::Query(page, 0), false);
            for i in 0..5000 {
                graph.touch(Key::Geometry(format!("{page}/{i}")));
            }
            graph.end();
            graph.finish(&page.to_string(), true);
        }
        graph.start();
        graph.touch(Key::Query(0, 0));
        assert_eq!(graph.finish("0", true).len(), 6 * 5001);
        graph.start();
        graph.begin(Key::Query(0, 0), false);
        graph.end();
        assert_eq!(graph.finish("0", true).len(), 5 * 5001 + 1);
        assert_eq!(
            graph
                .retain_files(&["0".into()].into_iter().collect())
                .len(),
            1
        );
        assert!(graph.retain_files(&HashSet::new()).is_empty());
    }
}

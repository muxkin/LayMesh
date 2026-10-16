//! Account shared immutable allocations once across retained query records.
//! The map contains identities/sizes only; the records remain the owners.
use super::*;
use crate::geometry_query::{Query, Step};
use crate::model::{Args, Object, V};
use serde_json::Value as J;
pub(crate) type Blocks = BTreeMap<(u8, usize), usize>;
#[derive(Default)]
pub(crate) struct Footprint {
    pub owned: usize,
    pub blocks: Blocks,
}
impl Footprint {
    fn block(&mut self, kind: u8, id: usize, size: usize, contents: impl FnOnce(&mut Self)) {
        let key = (kind, id);
        if self.blocks.contains_key(&key) {
            return;
        }
        self.blocks.insert(key, size);
        let before = self.owned;
        contents(self);
        *self.blocks.get_mut(&key).unwrap() += self.owned - before;
        self.owned = before;
    }
    pub fn resource(&mut self, bytes: &Arc<Vec<u8>>) {
        self.block(
            8,
            Arc::as_ptr(bytes) as usize,
            bytes.capacity() + 128,
            |_| {},
        );
    }
    pub fn style(&mut self, read: &Rc<crate::style::StyleRead>) {
        self.block(9, Rc::as_ptr(read) as usize, read.bytes() * 4, |_| {});
    }
    fn args(&mut self, args: &Args) {
        self.owned += args.len() * (std::mem::size_of::<V>() + std::mem::size_of::<String>() + 64);
        for (key, value) in args {
            self.owned += key.capacity();
            self.value(value);
        }
    }
    fn json(&mut self, value: &Arc<J>) {
        self.block(
            3,
            Arc::as_ptr(value) as usize,
            json_bytes(value) + 64,
            |_| {},
        );
    }
    pub fn object(&mut self, object: &Rc<RefCell<Object>>) {
        self.block(
            1,
            Rc::as_ptr(object) as usize,
            std::mem::size_of::<Object>() + 64,
            |this| {
                let o = object.borrow();
                this.owned += o.file.capacity() + o.kind.capacity();
                this.block(2, o.args.allocation(), 64, |this| this.args(&o.args));
                this.block(4, o.layers.allocation(), 64, |this| {
                    this.owned += o.layers.capacity() * std::mem::size_of::<(String, Args)>();
                    for (op, args) in &o.layers {
                        this.owned += op.capacity();
                        this.args(args);
                    }
                });
                this.block(
                    7,
                    o.origin_dependencies.allocation(),
                    64 + o.origin_dependencies.capacity() * std::mem::size_of::<(u64, u64)>(),
                    |_| {},
                );
                this.owned += o.nodes.capacity() * std::mem::size_of::<Arc<J>>();
                for node in &o.nodes {
                    this.json(node);
                }
                if let Some(node) = &o.node {
                    this.json(node);
                }
                if let Some(key) = &o.placed_key {
                    this.owned += key.len() + 64;
                }
            },
        );
    }
    fn query(&mut self, query: &Rc<Query>) {
        self.block(
            5,
            Rc::as_ptr(query) as usize,
            std::mem::size_of::<Query>() + 64,
            |this| {
                this.owned += query.steps.capacity() * std::mem::size_of::<Step>();
                for step in &query.steps {
                    match step {
                        Step::Member(name) => this.owned += name.capacity(),
                        Step::Index(value) => this.value(value),
                        Step::Call(name, values, args) => {
                            this.owned +=
                                name.capacity() + values.capacity() * std::mem::size_of::<V>();
                            for value in values {
                                this.value(value);
                            }
                            this.args(args);
                        }
                    }
                }
            },
        );
    }
    pub fn value(&mut self, value: &V) {
        match value {
            V::Object(object) => self.object(object),
            V::Number(_, unit) | V::Text(unit, _) => self.owned += unit.capacity(),
            V::List(values) => {
                self.owned += values.capacity() * std::mem::size_of::<V>();
                for value in values {
                    self.value(value);
                }
            }
            V::Map(args) => self.block(2, args.allocation(), 64, |this| this.args(args)),
            V::Dict(args) => {
                self.owned += args.capacity()
                    * (std::mem::size_of::<V>() + std::mem::size_of::<String>() + 64);
                for (key, value) in args {
                    self.owned += key.capacity();
                    self.value(value);
                }
            }
            V::Color(color) => self.block(
                6,
                Rc::as_ptr(color) as usize,
                format!("{color:?}").len() * 4 + 256,
                |_| {},
            ),
            V::Geometry(query) => self.query(query),
            V::Measured(query, value) => {
                self.query(query);
                self.owned += std::mem::size_of::<V>();
                self.value(value);
            }
            V::Anchor {
                reference: Some(r), ..
            } => self.owned += r.name.capacity(),
            V::Null | V::Bool(_) | V::Anchor { .. } => {}
            V::Cmap(_) => {} // The immutable registry is shared by every engine.
            V::Function { .. } => {
                debug_assert!(false, "Functions cannot enter retained contributions");
            }
        }
    }
}
pub(crate) fn json_bytes(value: &J) -> usize {
    std::mem::size_of::<J>()
        + match value {
            J::String(s) => s.capacity(),
            J::Array(a) => {
                a.iter().map(json_bytes).sum::<usize>()
                    + (a.capacity() - a.len()) * std::mem::size_of::<J>()
            }
            J::Object(m) => m
                .iter()
                .map(|(k, v)| k.capacity() + std::mem::size_of::<String>() + 64 + json_bytes(v))
                .sum(),
            _ => 0,
        }
}

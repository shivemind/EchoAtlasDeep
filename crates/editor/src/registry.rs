use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use core::ids::BufferId;
use crate::buffer::EditorBuffer;

pub struct BufferRegistry {
    buffers: HashMap<BufferId, Arc<RwLock<EditorBuffer>>>,
    next_id: u32,
}

impl BufferRegistry {
    pub fn new() -> Self {
        Self { buffers: HashMap::new(), next_id: 1 }
    }

    pub fn new_buffer(&mut self) -> BufferId {
        let id = BufferId::new(self.next_id);
        self.next_id += 1;
        let buf = EditorBuffer::new(id);
        self.buffers.insert(id, Arc::new(RwLock::new(buf)));
        id
    }

    pub fn new_buffer_from_file(&mut self, path: std::path::PathBuf, data: Vec<u8>) -> BufferId {
        let id = BufferId::new(self.next_id);
        self.next_id += 1;
        let buf = EditorBuffer::from_file(id, path, data);
        self.buffers.insert(id, Arc::new(RwLock::new(buf)));
        id
    }

    pub fn get(&self, id: BufferId) -> Option<Arc<RwLock<EditorBuffer>>> {
        self.buffers.get(&id).cloned()
    }

    pub fn remove(&mut self, id: BufferId) -> bool {
        self.buffers.remove(&id).is_some()
    }

    pub fn ids(&self) -> Vec<BufferId> {
        self.buffers.keys().copied().collect()
    }

    pub fn find_by_path(&self, path: &std::path::Path) -> Option<BufferId> {
        for (id, buf_arc) in &self.buffers {
            let buf = buf_arc.read();
            if buf.path.as_deref() == Some(path) {
                return Some(*id);
            }
        }
        None
    }
}

impl Default for BufferRegistry {
    fn default() -> Self { Self::new() }
}

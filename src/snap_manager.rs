use std::path::Path;

use crate::{dindex::DIndexVersionId, index_manager::DIndexManager};

struct FileVersion((String, DIndexVersionId));

struct Snapshot {
    entries: Vec<FileVersion>,
    id: DIndexVersionId,
}
impl Snapshot {
    fn from_directory_contents(target: String) -> Snapshot {}
    fn from_snap_file() {}
}
impl From<Snapshot> for String {
    fn from(snap: Snapshot) -> String {
        let mut string = String::new();
        for entry in snap.entries {
            let name = &entry.0.0;
            let id_str: [u8; 32] = entry.0.1.into();
            string.push_str(name);
            string.push_str(" ");
            string.push_str(&hex::encode(id_str));
            string.push_str("\n");
        }
        string
    }
}

struct SnapshotManager {
    snap_index_path: String,
    head_snap_path: String,
    data_index_manager: DIndexManager,
    snap_index_manager: DIndexManager,
}

impl SnapshotManager {
    const SNAP_INDEX_NAME: &str = "__snap_index";
    pub fn new(manager: DIndexManager) -> SnapshotManager {
        let snap_root = Path::new(&manager.data_root()).join("snaps");
        let head_snap_path = snap_root
            .join("HEAD")
            .into_os_string()
            .into_string()
            .expect("snap head path contains invalid unicode");
        let snap_index_path = snap_root
            .join("index")
            .into_os_string()
            .into_string()
            .expect("snap index path contains invalid unicode");
        SnapshotManager {
            snap_index_path,
            head_snap_path,
            data_index_manager: manager,
            snap_index_manager: DIndexManager::new(
                &snap_root
                    .into_os_string()
                    .into_string()
                    .expect("snap root path contains invalid unicode"),
            ),
        }
    }

    fn get_head(&self) -> Option<DIndexVersionId> {
        None
    }

    pub fn new_snap(&self, target: String) {
        let current = Snapshot::from_directory_contents(target);
        let parent_id = self.get_head().unwrap_or(current.id);
        self.snap_index_manager
            .insert(Self::SNAP_INDEX_NAME, &String::from(current), parent_id);
    }
}

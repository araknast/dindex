use crate::dindex::DIndexVersionId;
use crate::snap_manager::errors::SnapshotReadError;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Snapshot {
    entries: HashMap<PathBuf, DIndexVersionId>,
    parent_id: Option<DIndexVersionId>,
}
impl Snapshot {
    pub fn new(parent_id: Option<DIndexVersionId>) -> Snapshot {
        Snapshot {
            entries: HashMap::new(),
            parent_id,
        }
    }
    pub fn update_entry(&mut self, path: impl AsRef<Path>, id: DIndexVersionId) {
        self.entries.insert(path.as_ref().to_path_buf(), id);
    }
    pub fn contains_path(&self, path: impl AsRef<Path>) -> bool {
        self.entries.contains_key(path.as_ref())
    }
    #[cfg(test)]
    pub fn get_version_id(&self, path: impl AsRef<Path>) -> Option<&DIndexVersionId> {
        self.entries.get(path.as_ref())
    }
    pub fn into_entries(self) -> HashMap<PathBuf, DIndexVersionId> {
        self.entries
    }
    pub fn entries(&self) -> &HashMap<PathBuf, DIndexVersionId> {
        &self.entries
    }
}
impl From<Snapshot> for String {
    fn from(snap: Snapshot) -> String {
        let mut string = String::new();
        if let Some(parent_id) = snap.parent_id {
            string.push_str(&hex::encode(parent_id));
            string.push_str("\n");
        } else {
            string.push('\0');
            string.push_str("\n");
        }
        for (path, id_str) in snap.entries {
            let name = path.as_os_str().to_string_lossy();
            string.push_str(&name);
            string.push_str(" ");
            string.push_str(&hex::encode(id_str));
            string.push_str("\n");
        }
        string
    }
}

impl TryFrom<String> for Snapshot {
    type Error = SnapshotReadError;
    fn try_from(data: String) -> Result<Snapshot, Self::Error> {
        let mut entries = HashMap::new();
        let mut iter = data.lines();
        let parent_id_str = iter.next().ok_or(SnapshotReadError::EarlyTermination)?;
        let parent_id: Option<DIndexVersionId> = if parent_id_str == "\0" {
            None
        } else {
            Some(hex::decode(parent_id_str)?.try_into()?)
        };

        for line in iter {
            let mut split = line.split(" ");
            let name = split.next().ok_or(SnapshotReadError::EarlyTermination)?;
            let id: DIndexVersionId =
                hex::decode(split.next().ok_or(SnapshotReadError::EarlyTermination)?)?
                    .try_into()?;
            entries.insert(PathBuf::from(name), id);
        }

        Ok(Snapshot { parent_id, entries })
    }
}

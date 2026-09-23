# DIndex
## Design
### DIndexes

The DIndex is a data structure/file format for compactly storing multiple
different versions of a text file. It consists of a sequence of lines of text,
and a set of keys which allow a particular file version to be reconstructed
from these stored lines.

When a file is added to a DIndex, every line in the file is compared against
the lines in the stored sequence. Lines that do not already exist in the
sequence are added in the order in which they appear in the original file.
Finally, a key is generated for the file and stored in the DIndex. A key is a
sequence of subsequences of the line sequence which, when concatenated,
construct the original file.

A file entry in the DIndex consists of a file's key, as well as optional
pointers to a 'previous' and 'next' version. 


### DPacks
To optimize filesystem usage, several DIndexes may be combined into a DPack,
which is then compressed and stored to the filesystem. The creation and
accessing of DPacks is handled by the DPack manager. The manager maintains a
map describing which DPacks contain which DIndexes. When a particular index is
requested, the manager uses he map to locate the appropriate DPack, decompress
it, and return the appropriate DIndex.

## Interface
This library provides two main structs: `DIndex` and `DIndexManager`. 

### `DIndex`
`DIndex` is an in-memory representation of a DIndex that can be written to and
read from. 

Data for a particular version of a file can be inserted by calling
`DIndex::insert_version`, which returns a version id. This version id can then
be passed to `DIndex::get_version_data` to retrieve the stored data from the
DIndex.

The DIndex can be serialized and deserialized by calling `Vec<u8>::from` and
`DIndex::try_from` respectively.


### `DPackManager`
The DPack manager handles organizing `DIndex`s, as well as writing them and
reading them from the filesystem. All data will be written to the directory
passed in the `data_root` constructor argument. `DIndexManager::try_persist` is
used to to persist a `DIndex` object to the filesystem, and
`DIndexManager::try_load` is used to load a persisted DIndex back into memory.

## `snap` / `re`
`snap` and `re` are two demo programs for creating and reproducing DIndex data
respectively. The are used in `move.sh` and `verify.sh` respectively, but can
also be used on their own. 

`snap` takes as argument a target directory to snapshot, and a data directory in
which to store the snapshot data. The id of the generated snapshot is printed to
stdout.

`snap <target directory> <data directory>`

`re` takes as argument a data directory, an output directory, and a snapshot id.
If the specified snapshot exists in the data directory, it will reconstruct the
snapshot in the output directory.

`re <data directory> <snapshot id> <target directory>`

## move.sh / verify.sh
`move.sh` and `verify.sh` are a demo/integration test scripts for the DIndex
structure and DPack manager. 

`move.sh` takes as argument a git repository and an empty directory. Starting
at the first commit, the script will record each file into a DIndex. Once each
file in the repository has been recorded, the script will advance to the next
commit, and update the DIndexes with the new file contents. This process
continues until the HEAD commit is reached and the entire repository has been
migrated.

`./move.sh <repo path> <data dir>`

`verify.sh` takes as argument a git repository, a data directory produced by
`move.sh`, and an empty output directory. The script will walk through each
snapshot present in the data directory. Each snapshot is rebuilt in the output
directory, and compared against the git repository's worktree at the
corresponding commit. Any files that differ will be printed to console. By
default, the `.git` directory of the original repository is ignored.

`./verify.sh <repo path> <data dir> <output directory>`
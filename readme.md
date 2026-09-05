# DIndex

a data structure for tracking file versions

a DIndex holds version data for a single file

a DIndex is named and its name is the same as the name of the file it versions

## commands
`dindex put <file name> <data dir>` will store a file in a dindex and print out its version id

`dindex get <file name> <data dir> <version id>` will get a fille at a particular version

## snap.sh
for moving git repos into dindexes

`./snap.sh <repo path> <data dir>`

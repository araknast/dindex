#!/bin/sh

dindex=$(realpath ./target/release/snap)
dir="$(realpath $1)"
data="$(realpath $2)"
if test -z $dir || test -z $data
then
	exit
fi
prevdir=$PWD
cd $dir
git reset --hard origin/HEAD
for commit in $(git log --format="%H" --reverse); do
	git checkout "$commit"

	$dindex $dir $data
	
	du -bsh $data
done
cd $prevdir

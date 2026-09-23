#!/bin/sh

snap=$(realpath ./target/release/snap)
dir="$(realpath $1)"
data="$(realpath $2)"
snaplog="$data/snap.log"

if test -z $dir || test -z $data
then
	exit
fi
prevdir=$PWD
cd $dir
git reset --hard origin/HEAD
println "" > "$snaplog"
for commit in $(git log --format="%H" --reverse); do
	git checkout "$commit"

	echo "$($snap $dir $data) $(git rev-parse HEAD)" >> "$snaplog"
	
	du -bsh $data
done
cd $prevdir

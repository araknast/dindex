#!/bin/sh

re=$(realpath ./target/release/re)
original=$(realpath $1)
data="$(realpath $2)"
output="$(realpath $3)"
snaplog="$data/snap.log"

if test -z $snaplog || test -z $output || test -z $original
then
	exit
fi

prevdir="$PWD"
cd "$original"
git reset -q --hard origin/HEAD
cd "$prevdir"
while read line
do
	snap_id=$(echo $line | cut -f1 -d' ')
	commit_id=$(echo $line | cut -f2 -d' ')
	$re "$data" $snap_id "$output"
	cd "$original"
	git reset -q --hard $commit_id
	cd "$prevdir"
	diff -r "$output" "$original"
	rm -r "$output"
	mkdir -p "$output"
done < "$snaplog"
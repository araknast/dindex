#!/bin/sh

dindex="./target/release/dindex"
dir="$1"
data="$2"
if test -z $dir || test -z $data
then
	exit
fi
mkdir -p $data/bin
cd $dir
git reset --hard origin/HEAD
for commit in $(git log --format="%H"); do
	git checkout "$commit"

	cd ..
	for file in $(find $dir \( -type d \( -name ".git" -o -name "node_modules" \) -prune \) -o -type f -print)
	do
		if isutf8 -q $file; then
			$dindex put "$file" "$data" >/dev/null
		else
			cp "$file" $data/bin
		fi
	done
	du -bsh $data
	cd $dir 
done
zstd -q --rm $data/bin/*

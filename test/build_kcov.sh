#!/bin/bash
die() {
	echo >&2 "$@"
	exit 0
}

[[ -d "$HOME/kcov-master/build" ]] && cd "$HOME/kcov-master/build" && sudo make install && die "Directory already exists, refusing to build again"

mkdir -p "$HOME/kcov-master"
cd "$HOME/kcov-master"
wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz -O /tmp/kcov-master.tar.gz
tar xzf /tmp/kcov-master.tar.gz
cd kcov-master
mkdir build
cd build
cmake ..
make -j$(nproc)
sudo make install
cd ~/build/sameer/git-lfs-ipfs

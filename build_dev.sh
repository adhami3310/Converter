#!/usr/bin/env bash

BUILD_DIR="build/"
if [ -d "$BUILD_DIR" ]; then
	rm -r build
fi

mkdir build
cd build
meson ..
meson configure -Dprefix=$PWD/testdir -Dprofile="development"
ninja
ninja install

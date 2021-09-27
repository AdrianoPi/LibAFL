#!/bin/bash
wget https://deac-fra.dl.sourceforge.net/project/libpng/libpng16/1.6.37/libpng-1.6.37.tar.xz
tar -xvf libpng-1.6.37.tar.xz
cd libpng-1.6.37
./configure
make CC="$(pwd)/../target/release/libafl_cc" CXX="$(pwd)/../target/release/libafl_cxx" -j 12
cd ..
./target/release/libafl_cxx ./harness.cc libpng-1.6.37/.libs/libpng16.a -I libpng-1.6.37/ -o fuzzer_libpng -lz -lm -I/usr/lib/x86_64-linux-gnu/openmpi/include/openmpi -I/usr/lib/x86_64-linux-gnu/openmpi/include -pthread -L/usr/lib/x86_64-linux-gnu/openmpi/lib -lmpi
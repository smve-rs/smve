#!/bin/bash

cd $(dirname $0)

mkdir output
cargo run -p smve_asset_pack_compiler -- -a test_assets -o output/out.smap -u example_uncookers/e_uncooker.lua --no-default-uncookers

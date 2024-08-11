#!/bin/sh

cd $(dirname $0)

cargo run -p smve_asset_pack_compiler -- -a asset_group_1 -o asset_group_out/pack1.smap
cargo run -p smve_asset_pack_compiler -- -a asset_group_2 -o asset_group_out/pack2.smap
cargo run -p smve_asset_pack_compiler -- -a asset_group_built_in -o built_in.smap


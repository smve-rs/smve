#!/bin/sh

cd $(dirname $0)

cargo run -p smve_asset_pack_compiler -- -a asset_group_1 -o asset_group_out/pack1.smap -n
cargo run -p smve_asset_pack_compiler -- -a asset_group_2 -o asset_group_out/pack2.smap -n
cargo run -p smve_asset_pack_compiler -- -a asset_group_built_in -o built_in.smap -n
cargo run -p smve_asset_pack_compiler -- -a asset_group_override1 -o override1.smap -n
cargo run -p smve_asset_pack_compiler -- -a asset_group_override2 -o override2.smap -n

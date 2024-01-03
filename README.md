# Ruxel

A voxel engine written in rust.

## What does the name mean?
"Ruxel" is a [portmanteau](https://arc.net/l/quote/pnoxgupb) of the words "rust" and "voxel", because this project is a **voxel** engine written in **rust**.

## Plans
***This project is still in its infancy so there are very little features.***

The core engine will be an executable. You will be able to extend the engine with "mods" (they stand for modules, not
modifications as in Minecraft). Mods will be written in a scripting language like lua or rhai (haven't decided yet) and
they will subscribe to engine events such as `on_initialise` or `on_register_block` and they can call engine functions.

You can provide resources that a module uses (such as textures, audio, etc.) with asset packs which is in a custom-made
file format. It is a file that packages a directory of files into one file without compression.

The engine will provide default modules and asset packs for a minimal infinitely generating sandbox world and a menu
screen. You can select and enable mods and asset packs from the menu screen, and mods can also modify or completely
rewrite the menu screen.

Modules hopefully also will be able to be written in normal Rust and compiled into a library.

## Contributing

See [CONTRIBUTING.md](https://github.com/smgfx/ruxel/blob/master/CONTRIBUTING.md) for contributing guides.
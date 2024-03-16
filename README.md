<h1 style="text-align: center; line-height: 1">
  <img src="icon.png" style="width: 3em"><br>
  Ruxel
</h1>

<p style="text-align: center">A voxel engine written in rust.</p>

***

[![Stars](https://img.shields.io/github/stars/ItsSunnyMonster/ruxel?style=for-the-badge&logo=starship&logoColor=cdd6f4&labelColor=313244&color=f9e2af
)](https://github.com/ItsSunnyMonster/ruxel/stargazers)
[![License](https://img.shields.io/badge/license-MIT%2FApache-b4befe.svg?style=for-the-badge&labelColor=313244&logo=googleforms&logoColor=cdd6f4)](https://github.com/ItsSunnyMonster/ruxel#license)
[![CI](https://gist.githubusercontent.com/ItsSunnyMonster/a488eb0391a5fc6a2918d13184cd0a26/raw/ruxel_ci.svg)](https://github.com/ItsSunnyMonster/ruxel/actions)
[![Issues](https://img.shields.io/github/issues/ItsSunnyMonster/ruxel?style=for-the-badge&logo=gitbook&logoColor=cdd6f4&labelColor=313244&color=f5c2e7)](https://github.com/ItsSunnyMonster/ruxel/issues)
[![PRs](https://img.shields.io/github/issues-pr/ItsSunnyMonster/ruxel?style=for-the-badge&logo=git&logoColor=cdd6f4&labelColor=313244&color=fab387&label=PRs
)](https://github.com/ItsSunnyMonster/ruxel/pulls)

## What does the name mean?

"Ruxel" is a [portmanteau](https://arc.net/l/quote/pnoxgupb) of the words "rust" and "voxel", because this project is a
**voxel** engine written in **rust**.

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

See [CONTRIBUTING.md](https://github.com/ItsSunnyMonster/ruxel/blob/master/CONTRIBUTING.md) for contributing guides.

## License

Unless specifically stated, all code in this repository is dual-licensed under either

* MIT License ([LICENSE-MIT](https://github.com/ItsSunnyMonster/ruxel/blob/master/LICENSE-MIT)
  or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License ([LICENSE-APACHE](https://github.com/ItsSunnyMonster/ruxel/blob/master/LICENSE-APACHE)
  or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option. This means you can select the license you prefer.

### Your contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as
defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
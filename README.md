***

<h3 align="center" style="line-height: 1">
  <picture>
     <source media="(prefers-color-scheme: dark)" srcset="./images/title_logo_dark.svg">
     <source media="(prefers-color-scheme: light)" srcset="./images/title_logo_light.svg">
     <img alt="smve" width="200" src="./images/title_logo_light.svg">
 </picture>
</h3>

<p align="center"><strong>S</strong>unny<strong>M</strong>onster's <strong>V</strong>oxel <strong>E</strong>ngine</p>
<p align="center"><a href="http://ipa-reader.xyz/?text=%CB%88s%C9%9Bmvej">SEM-vay /ˈsɛmvej/</a></p>

***

<p align="center">
  <a href="https://github.com/ItsSunnyMonster/smve/stargazers"><img src="https://img.shields.io/github/stars/ItsSunnyMonster/smve?style=for-the-badge&amp;logo=starship&amp;logoColor=cdd6f4&amp;labelColor=313244&amp;color=f9e2af" alt="Stars"></a>
  <a href="https://github.com/ItsSunnyMonster/smve/issues"><img src="https://img.shields.io/github/issues/ItsSunnyMonster/smve?style=for-the-badge&amp;logo=gitbook&amp;logoColor=cdd6f4&amp;labelColor=313244&amp;color=f5c2e7" alt="Issues"></a>
  <a href="https://github.com/ItsSunnyMonster/smve/pulls"><img src="https://img.shields.io/github/issues-pr/ItsSunnyMonster/smve?style=for-the-badge&amp;logo=git&amp;logoColor=cdd6f4&amp;labelColor=313244&amp;color=fab387&amp;label=PRs" alt="PRs"></a>
  <br>
  <a href="https://github.com/smve-rs/smve"><img alt="Static Badge" src="https://img.shields.io/badge/github-smve--rs%2Fsmve-74c7ec?style=for-the-badge&logo=github&logoColor=cdd6f4&labelColor=313244"></a>
  <a href="https://crates.io/crates/smve"><img alt="Crates.io Version" src="https://img.shields.io/crates/v/smve?style=for-the-badge&logo=rust&logoColor=cdd6f4&labelColor=313244&color=f38ba8"></a>
  <br>
  <a href="https://github.com/ItsSunnyMonster/smve#license"><img src="https://img.shields.io/badge/license-MIT%2FApache-b4befe.svg?style=for-the-badge&amp;labelColor=313244&amp;logo=googleforms&amp;logoColor=cdd6f4" alt="License"></a>
  <img src="https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/ItsSunnyMonster/a488eb0391a5fc6a2918d13184cd0a26/raw/smve_loc.json" alt="Line of Code">
  <a href="https://github.com/ItsSunnyMonster/smve/actions"><img src="https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/ItsSunnyMonster/a488eb0391a5fc6a2918d13184cd0a26/raw/smve_ci.json" alt="CI"></a>
</p>

### ➜ Plans

***This project is still in its infancy so there are very little features, and I mean it.***
As of now, there is only a pipelined renderer drawing a blue window.

The core engine will be an executable. You will be able to extend the engine with "mods" (they stand for modules, not
modifications as in Minecraft). Mods will be written in a scripting language like lua or rhai (haven't decided yet) and
they will subscribe to engine events such as `on_initialise` or `on_register_block` and they can call engine functions
and interact with the ECS.

You can provide resources that a module uses (such as textures, audio, etc.) with asset packs which are .zip files with
a specific directory structure.

The engine will provide default modules and asset packs for a minimal infinitely generating sandbox world and a menu
screen. You can select and enable mods and asset packs from the menu screen, and mods can also modify or completely
rewrite the menu screen.

Modules hopefully also will be able to be written in normal Rust and compiled into a library.

### ➜ Contributing

See [CONTRIBUTING.md](https://github.com/ItsSunnyMonster/smve/blob/master/CONTRIBUTING.md) for contributing guides.

### ➜ License

Unless specifically stated, all code in this repository is dual-licensed under either

* MIT License ([LICENSE-MIT](https://github.com/ItsSunnyMonster/smve/blob/master/LICENSE-MIT)
  or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License ([LICENSE-APACHE](https://github.com/ItsSunnyMonster/smve/blob/master/LICENSE-APACHE)
  or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option. This means you can select the license you prefer.

#### ➜ Your contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as
defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

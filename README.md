<div align="center"> <img src="./embryo.png" width="128"
  height="128" style="display: block; margin: 0 auto"/>
  <h1>Embryo Engine</h1> <p>An intensely data-driven game
  engine</p>

<a href="https://www.flaticon.com/free-icons/embryo"
title="embryo icons">Embryo icons created by kerismaker -
Flaticon</a> </div>


---

## 🚧 Status: under construction 🚧

**Disclaimer**: This engine is in *very* early alpha. Basic
versions of foundational engine systems, such as the renderer,
game object system, event system, and so on, are still being
implemented and wired up or even just mapped out in code. As
such, all descriptions of the engine found here or anywhere else
are to be interpreted as design philosophy, guiding principles,
long term goals, or part of the general (but fairly detailed)
design I've mapped out before starting the project, which are
subject to change in the specifics, since I still have a lot to
learn about what the concrete implementations of things will look
like, which can change final design parameters.

## The pitch

Do you look back fondly on the immersive sims of Looking Glass
Studios[^1] and Ion Storm[^2] like *Thief*, *System Shock 2*, or
*Deus Ex*? Or the early open-world RPGs of Bethesda Game Studios
like *Oblivion* and especially *Morrowind*? Do you wish more
games like those existed, or do you want to make one of your own?

Do you find yourself wishing for a game engine that is leaner, more genre-focused, and easier to fully understand and even modify than mainstream offerings? Are you finding yourself fighting with mainstream offerings to achieve the things you want, or finding that their tools are too bloated, GUI-focused, confusing, and slow? Do you crave the cold certainty of ~~steel~~ plain text? Then this game engine is for you.

Despite their dedicated, passionate fanbases and their wide open markets created by a lack of big new titles, both the open-world sandbox RPG and immersive sim genres remain criminally under-served, largely because, in both cases, the fanbases that will buy these games are just not big enough to sustain a large game studio or publisher most of the time. This means that if we want more of these games, it is indie developers and studios that will have to make them. The problem with this is that both of these genres come with serious technical challenges that make implementing them with generic tools like Godot, Unreal, or Unity a daunting task.

Since the technical challenges of both genres overlap substantially, the Embryo Engine aims to be laser-focused on providing the best possible game engine for developing precisely these two genres of games and nothing else. Every design decision for Embryo is made from the ground up, from algorithms to engine architecture, with maximizing its potential for developing these genres in mind. With the recent boom of indie immersive sims[^3], and the proven possibility of making convincing open-world indie games without the absurd budget and content requirements of AAA open-world titles[^4], I hope that the Embryo Engine will contribute to this trend and help people make more of the kinds of games I want to see!

## Core design ideas

- 📝 **Fully data-driven[^5], powered by WebAssembly**: 100% of game content should be expressible using plain data files and scripts compiled to WASIX[^8], with only actual behavior requiring scripts while everything else is specified as data. Game scripts and data should be strictly separate from the game engine, without linking to the engine like a framework or modifying it. Moreover, instead of game data and scripts being compiled along with the game engine into a single binary blob for release, the game engine should remain a distinct entity that acts as an interpreter for the game data, when run as the game's executable. In theory, players should be able to completely swap out your game's game engine executable for another one and everything should still work.[^6]

- 🔬 **Intentionally moddable**: all game data should use open
  and standardized formats, and where possible these formats
  should also be plain-text and human-readable, so that as little
  special software as possible is needed to create or modify game content. Additionally, game content should be easy to replace or modify even in existing games. For instance, you should be able to easily add new entity prefabs by just dropping a file into the relevant folder, and instantiate new entities by adding lines to the relevant world/location files, or even replace prefabs or world files entirely by replacing their files, and the changes should be predictibly reflected when running the game.
  
- 💥 **Emergent sandbox simulation focused**: this game engine should try to maximize the possibilities for simulation and emergent behavior as much as possible through a sparse set entity component system, to make dynamically mixing and matching game object properties and behaviors at runtime possible, and a flexible event and messaging system based on Caves of Qud's system and Erlang's actor model, increasing the ways in which behavior can combine while minimizing tight coupling or the need to predict such combinatorial emergent behavior.

- 🧓 **Old things, new tech**: this game engine will target
  roughly 1998-2005 level computer graphics, sound, and VFX capabilities, while making the most of modern hardware and productivity-enhancing tools like Rayon, OpenGL 4.6 ([why?](https://github.com/alexispurslane/embryo-engine/wiki/Why-OpenGL-and-not-Vulkan%3F)), Rust, and SDL 2.0. While big game studios scale the difficulty of their ambitions up to match the increased productivity of modern tooling, indies turn that increased productivity to things with a constant level of difficulty determined by past technological constraints in order to compensate for smaller teams and budgets. This engine will employ that same strategy. Additionally, the tradeoff between artistic expressiveness (not visual fidelity) and algorithmic complexity peaked between roughly 1998-2005, and most of that time's constraints on expressiveness were the results of hardware limitations, not software ones. Therefore, with modern hardware, older, simpler, and more maintainable algorithms will be perfectly acceptable.

- 🔀 **Maximize parallelism and minimize overhead**: despite the old warning against premature optimization, in order to make my engine as scriptable as possible, it needs the core engine to be low-overhead and fast, to leave as much frame time for scripting as possible; likewise, to maximize its ability to allow possibly computationally intensive simulation behavior, the core engine needs to be well-optimized. Therefore, a lot of design effort, thinking, and research has gone into choosing the best algorithms, data structures, and general engine architecture to make this engine as performant as possible *for its specific use-case in immersive sims and open world RPGs*. This includes making the engine as parallel as possible without making it an infeasible task, utilizing insights from AAA game engine development.

For a more in-depth dive into the design choices for this engine
and the reasoning behind them, check out [the high-level overview on the wiki](https://github.com/alexispurslane/embryo-engine/wiki). Although most of my design notes are in
my physical notebook or in my head (which is why I won't be
accepting contributions or pull requests until at least the basic
architecture of the engine is completed), I thought it might be
useful to have such a document so others can get a sense for what I'm going for.

## Progress

### Rendering

- [x] Load and render glTF 3D models with textures and materials
- [x] Runtime conversion of PBR to Blinn-Phong materials[^7]
- [x] Dynamic ambient, diffuse, point, and spot lights with all the tunables
- [x] Deferred shading and lighting pipeline with model instancing and batching and light bounding volumes
- [x] Transforms, Models, Lights, and Cameras are all components with tunable parameters
- [x] Camera pulls position from transform component on same entity
- [x] HDR rendering, fairly advanced tonemapping
- [x] Font rendering (not quite good, but renders any TTF you want)
- [x] Transform hierarchies
- [ ] Basic UI elements
- [ ] Shadows
- [ ] Bloom and fog
- [ ] Normal maps
- [ ] Emissive textures
- [ ] Frustum culling of instances using geometry shaders
- [ ] Display heightmaps using tessellation shaders
- [ ] Antiportal culling
- [ ] Skyboxes
- [ ] Mirrors
- [ ] Transparency
- [ ] Caching
- [ ] Particle effects

### General architecture

- [x] Opens window with SDL2, displays renderer output
- [x] Basic naive ECS with generational references fully implemented (can create new entities, delete old entities, reuse old entity IDs and detect when an entity reference is out of date, add and remove components from entities, have a few basic components) with test entities created
- [x] Full asynchronous function parallel threading model with resource loader, renderer, event loop/window handler, and update loop
- [x] Asynchronous thread-pool based resource loader and converter integrating with renderer and update loop
- [x] Basic keyboard controls
- [x] Relays events to update loop, can respond to user input to move the player entity around
- [ ] Implement proper sparse set ECS
- [ ] Add heightmap component
- [ ] Implement event dispatch system and event listener registry
- [ ] Implement WebAssembly Minimal Runtime based background (single-threaded) system scripts
- [ ] Introduce actor-targeted events
- [ ] Implement WebAssembly Minimal Runtime based data parallel event forwarding actor component behavior script pipelines
- [ ] Implement realtime ECS backtracking query system[^9]

### Resource Management(?)

- [ ] Quadtree scene graph for spacial queries
- [ ] Implement loading initial entities and world map from TOML configuration files
- [ ] Implement unloading scenes or models
- [ ] Implement auto chunking scenes as a build step for assets
- [ ] Implement dynamic world streaming using the asynchronous threaded resource manager

[^1]: <https://en.m.wikipedia.org/wiki/Looking_Glass_Studios>

[^2]: <https://en.wikipedia.org/wiki/Ion_Storm>

[^3]: <https://www.youtube.com/watch?v=SQRROIA6UQ8> and
    <https://www.youtube.com/watch?v=IwkDATs5NKo>

[^4]: See the GDC 2020 Virtual Talk by Adam Robinson-Yu,
    'Crafting A Tiny Open World: A Short Hike Postmortem'
    <https://www.youtube.com/watch?v=ZW8gWgpptI8> and this video:
    <https://www.youtube.com/watch?v=S3cPJL4ISlU>

[^5]: Not to be confused with "data oriented," which is what
    engines like Bevy and Amathyst mean when they say
    "data-driven," despite the concepts being very different. See
    this talk to understand what data-driven means for engines:
    <https://gdcvault.com/play/1022543/A-Data-Driven-Object>.

[^6]: Think something like this:
    <https://taleoftwowastelands.com/faq>, but even better.
    
[^8]: https://wasix.org/
[^7]: https://developer.valvesoftware.com/wiki/Adapting_PBR_Textures_to_Source

[^9]: https://ajmmertens.medium.com/a-roadmap-to-entity-relationships-5b1d11ebb4eb

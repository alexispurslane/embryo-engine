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
  roughly 1998-2005 level computer graphics, sound, and VFX capabilities, while making the most of modern hardware and productivity-enhancing tools like Rayon, OpenGL 4.6[^7], Rust, and SDL 2.0. While big game studios scale the difficulty of their ambitions up to match the increased productivity of modern tooling, indies turn that increased productivity to things with a constant level of difficulty determined by past technological constraints in order to compensate for smaller teams and budgets. This engine will employ that same strategy. Additionally, the tradeoff between artistic expressiveness (not visual fidelity) and algorithmic complexity peaked between roughly 1998-2005, and most of that time's constraints on expressiveness were the results of hardware limitations, not software ones. Therefore, with modern hardware, older, simpler, and more maintainable algorithms will be perfectly acceptable.

- 🔀 **Maximize parallelism and minimize overhead**: despite the old warning against premature optimization, in order to make my engine as scriptable as possible, it needs the core engine to be low-overhead and fast, to leave as much frame time for scripting as possible; likewise, to maximize its ability to allow possibly computationally intensive simulation behavior, the core engine needs to be well-optimized. Therefore, a lot of design effort, thinking, and research has gone into choosing the best algorithms, data structures, and general engine architecture to make this engine as performant as possible *for its specific use-case in immersive sims and open world RPGs*. This includes making the engine as parallel as possible without making it an infeasible task, utilizing insights from AAA game engine development.

For a more in-depth dive into the design choices for this engine
and the reasoning behind them, check out [my general design
document](./DESIGN.md). Although most of my design notes are in
my physical notebook or in my head (which is why I won't be
accepting contributions or pull requests until at least the basic
architecture of the engine is completed), I thought it might be
useful to have such a document so others can get a sense for what
I'm going for.

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
    
[^7]: I'm not using Vulkan because, even compared to AZDO OpenGL
    4.6, it is significantly lower level, functioning more as a
    GPU driver than a graphics driver, out of which you have to
    essentially *build your own* graphics driver and manage your
    own state. This is helpful for teams of industry professional
    game engine programmers, as this lower-level access means
    many more optimizations can be taken advantage of if you have
    the scale, time, resources, and knowledge, but Vulkan only
    *raises the performance ceiling*, it doesn't make regular
    graphics programming for most of us any faster, so the
    benefits of Vulkan are mostly out of reach for hobbyist and
    individual developers, whereas the detriments of Vulkan's
    much lower level are immediately felt in productivity across
    the board. Moreover, many of the things Vulkan can do that
    *are* acessible to regular developers, that used to be a big
    selling point, are now possible via modern OpenGL anyway,
    like direct state access. Thus, in the end, despite the hype
    around Vulkan, it really isn't right for most of the game
    industry. For those who want to program our own fairly
    complex, large scale rendering engines, OpenGL is still far
    better thanks to being significantly higher level. In
    essence, OpenGL is C to Vulkan's assembly language. In fact,
    it is this author's opinion that if OpenGL is ever "phased
    out" it will be a serious blow to the hobbyist game engine
    developer community, at least until another graphics
    programming and GPU programming API that lies at that same
    sweet spot of abstraction between Vulkan and a fully fledged
    rendering engine like OGRE or a game engine like Unity comes
    along. Perhaps that will be WebGPU (I'd like to think it will
    be), but in my opinion the documentation (both first party
    and community) around WebGPU simply isn't there yet. There is
    no WebGPU Red Book (or Blue Book for that matter), *Real-Time
    Rendering* doesn't explicitly relate what it's talking about
    to WebGPU concepts, etc. Until that point, OpenGL will be
    what this engine uses.
[^8]: https://wasix.org/

<div align="center">
  <img src="./embryo.png" width="128" height="128" style="display: block; margin: 0 auto"/>
  <h1>Embryo Engine</h1>
  <p>An intensely data-driven game engine</p>

<a href="https://www.flaticon.com/free-icons/embryo" title="embryo icons">Embryo icons created by kerismaker - Flaticon</a>
</div>


---

## 🚧 Status: under construction 🚧

**Disclaimer**: All possible descriptions of this engine at this point are aspirational at best.
This is not remotely ready to be downloaded and messed around with, let alone
used for anything. Most of the core features and "selling points" of this engine
are not even implemented.

## Core design ideas

- 📝 **Truly data-driven**: as close as possible to 100% of all game-specific logic and
  content should be able to be placed in simple data files and scripts, strictly separate from the engine, to the
  degree that in theory players should be able to replace the game engine
  executable in the home folder of any game made with this engine with a freshly-compiled
  version, and the game should still work (for the most part).
  This will make it easy to create large and complex game worlds with lots of
  content. **Note that due to a confusion in terminology on the part of others, this is very different from the kind of "data driven" that Bevy and Amathyst are — they mean "data driven" in the sense of data-oriented design,[^6] not actual data-driven design.**

- 🔬 **Explicitly, maximally moddable**: game data should be as easy, accessible, and flexible to
  create, edit, and even replace or add to an existing game, as possible. Game data should be dynamically replacable as much as possible, instead of compiled or hardcoded
  into the executable binary. This, combined with the previous point, means that essentially *all* game content will be "mods" --- both first party content and
  third party content. This means both that creating game content is faster, and that third party modders of these games will get unprecedented power, since their mods will have access to the full power of the complete game content creation API the engine offers, instead of a limited set of interface APIs for predefined tasks that the game developers figured mods should be able to do. This engine essentially wants to be the Emacs to most other engines' Vim --- Emacs is a relatively small core of performant C that exposes almost all of its behavioral internals for control by a dynamic scripting language, and almost all of the actual editor is built atop that using that scripting language, which means that Emacs plugins can be much more powerful and involved, even replacing core parts of the editor, because the editor itself is fully exposed to, and mostly written in, the scripting language. Whereas for instance Vim has a predefined and very separate plugin interface that basically only allows essentially automated keystrokes.
  
- 💥 **Emergent sandbox simulation focused**: this game engine should try to maximize the possibilities for
  simulation and emergent behavior as much as possible through an entity component system, to make dynamically mixing and matching game object properties and behaviors at runtime possible, and a flexible event
  and messaging system based on Caves of Qud's system and Erlang's actor model,
  increasing the ways in which behavior can combine while minimizing tight coupling
  or the need to predict such combinatorial emergent behavior. This will
  enable more complex, responsive, sandbox game worlds.
  
- 🧓 **Do old things with new tech**: doing any, let alone all, of
  this will be possible for a single person only through leveraging modern
  development tools to the utmost. But of course that wouldn't help if the game engine's goals scaled up to modern levels in tandem, since program complexity and difficulty in game engine development
  has scaled probably *faster* than our tools have. Therefore, I've picked a
  *past* era of video game technology I deem "good enough" to allow for sufficient creativity and expressivity in the content for the kinds of
  games I want to make and play, and will be rigerously and singlemindedly
  pursuing that. Therefore, I'm using Rust instead of C++ to save myself
  segfaults and memory leaks, I'm using OpenGL 4.6 with AZDO techniques instead
  of Vulkan[^5], I'm using preexisting physics middleware, and I'm targeting a hybrid of the level of technology of 7th-generation
  (2005-2010 or so) games and earlier PC games (1998-2000 or so).

## Motivation

Why make *yet another* game engine?

Part of it is simply the fact that I find game engine and computer graphics
programming fascinating --- those fields basically fully exercise every aspect
of software development as a whole, from large scale software architecture to
computer science (algorithms, data structures, mathematics) and everything in
between, while at the same time being rather immediately and tangibly rewarding
for all that hard work.

The greater part of my motivation, however, is to be found in the genres of  Ion Storm and Looking Glass Studios-style immersive sim games and Bethesda-style open-world RPGs. These games are so interesting and unique compared to others on the market, and even more interestingly, despite being different genres, the reasons for their uniqueness, as well as their unique technological requirements, are kind of shared if you think about it! This got me thinking about how I might go about creating a game engine specifically designed to enable the creation of these two interesting genres of games, and whether that might actually encourage people to make more of them, since they're pretty rare! There aren't really any engines
specifically targeting these genres currently, despite the fact that such games have very unique challenges to go with their unique merits and so would
definitely stand to benefit from a game engine architected around their needs and constraints. Maybe if there were such an engine, more such games would exist -- perhaps all
that is necessary for small-studio Bethesda-style open world RPGs to crop up is an engine that
accelerates development of such games! After all, almost no one is truly serving
that market. Likewise for immersive sims.

How could an engine focus on enabling such games, though?

Well, one of the things that truly makes *Bethesda's* games special is their
engine, NetImmerse / Gamebryo / Creation Engine. Despite the bad rap it gets ---
all of it largely undeserved, since it is not really any older than any other
mainstream engine, such as Unreal, has been used to make many other perfectly
good games outside of Bethesda, and is not really more fundamentally flawed than
other engines are (compare with Unreal struggling to take advantage of modern multicore
processors because of its outdated parallel processing architecture) --- it is largely what is responsible for the unique gameplay character and feel of their games: far more simulation heavy and dynamic than other games. Likewise, its specific design approach is probably partly responsible for Bethesda's ability to put as much sheer content in their games as they do, and certainly responsible for the singular moddability of their games. There are
several lessons to be learned from Gamebryo, which I will explore more below.

The lesson to learn from immersive sims, on the other hand, are more focused around design requirements, not specific technical aspects, but one does directly lead to another. To enable immersive sim style gameplay as much as possible, game object behavior and properties need to be extremely modular and encapsulated, to avoid overwhelming complexity, dynamically composable, to allow for great runtime freedom, and capable of influencing other properties and especially behavior *without explicit coupling*, so unforseen combinations can occur. Luckily, there are systems used in the *System Shock*, *Thief*, and *Caves of Qud* games that can help us with this. For more see below.

## Deeper exploration of design principls

### Data-driven design

The most unique thing about the Gamebryo engine is that it has an extremely
data-driven[^1] design. Essentially this means that all of the game-specific
logic and content, from the game world to the game objects, NPCs, quest systems,
game win and loss states, game mechanics, actor behavior, and more, are
specified using plain old data and a simple scripting language, which the game
engine then picks up and runs much in the same way VLC plays a movie, instead of
much of the game-specific code having to be written in the game engine's
language itself and/or statically linked to the engine. This is to *some* degree
similar to how many modern engines, such as RedEngine, Unreal, and Unity, act as
one monolithic structure that is then controlled via a scripting language, but
substantially different in practice. It is a matter of degree, not kind, but it
is a very large degree of difference nonetheless. There is a reason Bethesda's
own first-party DLC content for their games takes the exact same form that large
community content mods do, which cannot be said for any other engine.

This data-driven design has several benefits. First, it makes adding content to
Gamebryo engine games much less time consuming, more flexible, and simpler,
making large, content-rich, interconnected worlds more possible. Second, by
making a system by which one can easily and declaratively add new objects and
even mechanics to the game, and creating a scripting system with full access to
all of the engine's capabilities and a structure that allows adding new scripts
and removing old ones even after the game has been "compiled," this data-driven
architecture enables a much more vibrant and empowered modding community, since
first-party content and mods are essentially on a level playing field -- they're
both doing the same thing, meaning mods have the full power game developers also
have access to.

A highly data-driven architecture also has the further (theoretical) benefit
that if sufficient compatability is maintained between the data formats old and
new versions of the engine accept, old games can be upgraded with new versions
of the engine at any time with minimal need to actually edit the game!

The Embryo game engine aims to take this even further, by ensuring that all the
data files used by the engine are in readily readable/writable open standards,
instead of odd proprietary or in-house formats. Thus:

- for configuration files, initial game world specifications, game object
  (including actor) specifications, materials, and more, TOML files will be used
- for compiled game world files, game world chunks, and save files, MessagePack
  will be used
- for textures, simple image formats will be used
- for heightmaps, BMPs will be used (in a specific way)
- for 3d objects and parts of scenes, glTF 2.0 will be used

This way, anyone will be easily able to create or modify game files without any
specific suite of tools.

### Simulation-heavy and object-flexible

> In all of my universe I have seen no law of nature, unchanging and inexorable.
> This universe presents only changing relationships which are sometimes seen as
> laws by short-lived awareness... If you must label the absolute, use it's
> proper name: Temporary.
> 
> --- **The Stolen Journals**

One of the other interesting aspects of Gamebryo is the fact that almost
everything that exists in the game is simulated, to a much greater degree than
in other game engines. For instance, NPCs are fully as detailed as players.
Inventory items all have meshes and rigidbodies, instead of being etherial
powerups, so they can act like real objects in the world. Many things in scenes
can be picked up and moved around arbitrarily at will. All actors have ragdoll
skeletons as well as animated ones. And so on. This is part of what gives
Bethesda's games their unique flavor: unlike many other games of similar size
and scope, the way you interact with the world is not limited to a few specific
things you can do, outside of which the game will revert to an inert and
lifeless rock despite all your tugging and prodding. In a sense, Bethesda games
are much more of a simulated sandbox than other titles.

Of course, there is a drawback to this --- there are ***good reasons*** other
studios don't follow in Bethesda's footsteps. Namely, that as you increase the
simulational aspect of your game, you lose direct control over how the game
behaves, opening yourself up to many more bugs and restricting your ability to
tightly script, pace, and act things in your games. This is why while other
games like Cyberpunk 2077 have breathtaking in-game cutscenes, even Bethesda's
latest and least buggy game, Starfield, struggles to animate its characters
through an emotional scene without the physics engine getting in the way.

Nevertheless, there is a crucial spot in the gaming world for such sandbox style
games. In fact, there's a whole genre built around the idea that everything in
the game world should be simulated and responsive to any reasonable thing you
might want to try: immersive sims. From *System Shock 2* to *Deus Ex* to *Thief*, the key appeal of immersive sims is that you are given a large amount of powerful
tools, and set loose on a problem in an interesting, endlessly responsive environment, to solve it
however you like.

It may not seem obvious how to create a game engine that enables and accelerates
the development of such simulation-heavy games, besides perhaps mandating that
every game object have a mesh and a rigid body (a bad idea), but in watching
gameplay from Deus Ex and Pray, as well as simulation-focused roguelikes like
Dwarf Fortress and Caves of Qud[^2], a few things become clear. First of all,
you need to be able to dynamically add and remove properties and behaviors from
objects in combinations and at times that are not predictable ahead of time, and that to manage the complexity of such an endeavor, behavior and properties must be decoupled, encapsulated, and composable;
second of all, that those behaviors, as well as individual game objects and
actors, need to be able to effect each other's behavior without prior
expectation of being able to do so; and finally, that specifying these packages
of behavior and properties be as composible and declarative as possible. Let's look at each of these in turn.

- The first point could be simplistically enabled by just using a single game
  object class for everything in the entire game, so that it contains all the
  possible properties and behaviors any game object in the game could possibly
  possess, such that producing any given behavior or combination thereof could
  be produced in an object by just toggling on or off previously dormant
  properties. The problem with this is, of course, that it introduces a lot of
  coupling, problematic amounts of state management, and the possibilty of
  undesired interactions between properties, as well as wasting a lot of memory
  and probably being difficult to maintain. An easier way is to use a simple
  entity component system. This way, entities are just columns in a big table,
  and components can be easily and dynamically added and removed from entities
  as needed, where components represent compsible and encapsulated units of game object properties and behavior is desired (and behavior itself is separated).

- One of the problems with the design of a classic entity component system,
  however, is that since all object behavior is defined in terms of systems,
  which loop through all the entities with the necessary properties to have a
  behavior and perform that behavior for each entity, communication between behaviors or systems is difficult, and all combinatorial behavior
  must be specified up front: if I want a new behavior to emerge when an entity
  has two components at the same time, I have to either program that behavior
  into one of the existing systems for those components, or create a new system
  that operates only on entities with both components. Thus I must be able to
  predict and architect all possible combinations of system behavior. This is
  especially true as a result of the fact that it is difficult for systems to
  pass per-entity temporary information --- events and messages --- to other
  systems in an architecturally clean and encapsulated fashion, and difficult
  for them to manipulate the behavior of other systems from afar with those messages without tight
  coupling between them, because systems work on the basis of regular behaviors, not event-based ones. Many architectural questions pop up when trying to
  figure this out. Which system handles interlocking behavior? How does one
  system modify the information another acts on, without modifying the entity
  itself? Does the system generating the information modify the entities the
  information is directed at? That produces tight coupling and requires more
  foreknowledge about possible combinations. Furthermore, with a classic
  system-oriented structure, parallel processing becomes more difficult: if you
  have a series of systems that need to run on a group of entities in order, you
  have to run the first system on all the entities, then the next, and so on.
  However, the system might finish the step for some of those entities sooner
  than others, in which case it would be desirable for those entities which
  finished early to be able to move on from that step and work on other steps
  while the late ones are on the last one. This is impossible in a classic ECS
  system, which essentially requires a scatter/gather structure with a sync point between each behavior, with at best a few unrelated systems running at the same time. This is precisely the sort of design that Unreal had that causes it to struggle to make use of modern multicore CPUs. Likewise, passing information back and forth between threads in a
  classic ECS structure is difficult. Thus I borrowed a concept from Erlang:
  message-passing and Actor-oriented programming[^3], and a system from
  Naughty Dog called fibers[^4]. In this model, there is a pool of operating
  system threads, one per core, and jobs (or "fibers") that thread through one complete pipeline task that needs to be performed in order are generated for any processing that needs to be done that can
  be done in parallel, including actor behavior, and pushed onto a job stack
  which the threads then pull from whenever they're finished any previous jobs
  and begin working on. Specifically, in this case, all the behavioral
  processing necessary for each game object and actor is represented as a linear
  pipeline of transformations to the entity which can be performed independently
  of any of the other entities, just based on last frame's game state (as this
  process is producing this next frame's game state). Thus, each actor can
  update as fast as it can, proceeding through the pipeline. (This is actually
  related to how modern programmable pipelined GPUs work.) When they need to
  communicate, they send messages to either a global queue by message type,
  which each actor can subscribe to (such global messages are called "events")
  or they send that message to an actor's unique queue. General events in the
  world, such as collisons, entities entering trigger areas, update ticks, user
  input, will be distributed to all actors via this messaging and event system
  as well, meaning that *all actor behavior is triggered by events/messages*,
  including stuff that happens on each frame. How this works is inspired by the
  architecture of Caves of Qud's message based ECS, where events are fed to the
  first behavior on an object (according to a priority system), and that
  behavior can choose to act on any, all, or none of those events, and then
  *modify those events* or produce new ones (either to pass on, or to send to
  other entities, or to notify the game state that something at a higher level
  needs to change), and this new set of events is then passed to the next
  behavior. Thus, behaviors can modify each other by modifying the events they
  receive, since all behavior is described by and triggered by events, without
  any coupling whatsovever. I highly recommend you watch the talk in the
  footnotes if you want to learn more about this.
  
- As for our third and final point, easy game object and behavior/property
  specification and assembly in a declarative manner, that is already covered by
  having a data-driven design.

### Knowing where to draw the line

> Arrakis teaches the attitude of the knife--chopping off what's incomplete and
> saying: "Now, it's complete because it's ended here."
> 
> --- **from "Collected Sayings of, Muad'Dib" by the Princess Irulan**

The final principle of design I want to discuss here is less derived from
immersive sims and the Gamebryo engine, and more one derived from necessity: in
an attempt to limit the scope and scale of this endeavor to something relatively
more reasonable, at least at the outset, I've set a specific era of games in
mind that I want my engine's graphical, animation, and similar capabilities to
be able to match, and beyond that, I'm not going to worry about it, besides
making the engine extensible so that it's a good platform for doing more
advanced things if people want to. Everything is a tradeoff between benefit and
complexity, and for my limitations as an individual programmer, I've found that
the graphics algorithms and similar capabilities of 7th generation games seem to
be at the sweet spot of that tradeoff for me: any increase in capability
increases complexity at a vastly disproportional rate compared to the actual
tangible gains received (despite the gains being *very* large!), whereas any
decrease in capability diminishes the complexity of this endeavor only slightly,
while walling off large portions of game expressiveness --- the types of games
and visuals that can be made with the engine's technology. The era of Oblivian
and Fallout 3 is the first era in gaming where really big, large scale, fairly
immersive worlds could be crafted with many simulation elements, but before the
industry got quite as big and mind-bogglingly advanced as it is today.

Of course, I'm not blindly aiming for one level of technology. I'm using more
advanced data-oriented design, more modern graphics programming techniques and
APIs, and most importantly of all, the actual software architecture
underpinnings of my engine, like its object model and multiprocessing model, are
as advanced as I can feasibly make them, meaning that although it may look old
on the surface, this engine will have fresh, shiny, and well thought through,
modern internals. It's a sleeper, in effect.

Of course, this means it probably isn't for everybody, or even anybody, but,
well. I'm making the program ;)

[^1]: Not to be confused with Data-Oriented, although this engine is that too.
    See this talk: <https://gdcvault.com/play/1022543/A-Data-Driven-Object> or
    *Game Engine Architecture, 3rd Edition*.

[^2]: This talk is an especially good summary of how data-driven design, a
    flexible entity-component system, and a message-passing based event system
    that bubbles events up through the components on each entity much like
    events bubble up through DOM elements, is an especially good one for
    understanding what I'm talking about here:
    https://www.youtube.com/watch?v=U03XXzcThGU

[^3]: https://en.wikipedia.org/wiki/Actor_model

[^4]: https://www.gdcvault.com/play/1022186/Parallelizing-the-Naughty-Dog-Engine

[^5]: This is because Vulkan, even compared to AZDO OpenGL 4.6, is significantly
    lower level, functioning more as a GPU driver than a graphics driver, out of
    which you have to essentially *build your own* graphics driver and manage
    your own state. This is helpful for teams of industry professional game
    engine programmers, as this lower-level access means many more optimizations
    can be taken advantage of if you have the scale, time, resources, and
    knowledge, but Vulkan only *raises the performance ceiling*, it doesn't make
    regular graphics programming for most of us any faster, so the benefits of
    Vulkan are mostly out of reach for hobbyist and individual developers,
    whereas the detriments of Vulkan's much lower level are immediately felt in
    productivity across the board. Moreover, many of the things Vulkan can do
    that *are* acessible to regular developers, that used to be a big selling
    point, are now possible via modern OpenGL anyway, like direct state access.
    Thus, in the end, despite the hype around Vulkan, it really isn't right for
    most of the game industry. For those who want to program our own fairly
    complex, large scale rendering engines, OpenGL is still far better thanks to
    being significantly higher level. In essence, OpenGL is C to Vulkan's
    assembly language. In fact, it is this author's opinion that if OpenGL is
    ever "phased out" it will be a serious blow to the hobbyist game engine
    developer community, at least until another graphics programming and GPU
    programming API that lies at that same sweet spot of abstraction between
    Vulkan and a fully fledged rendering engine like OGRE or a game engine like
    Unity comes along. Perhaps that will be WebGPU (I'd like to think it will
    be), but in my opinion the documentation (both first party and community)
    around WebGPU simply isn't there yet. There is no WebGPU Red Book (or Blue
    Book for that matter), *Real-Time Rendering* doesn't explicitly relate what
    it's talking about to WebGPU concepts, etc. Until that point, OpenGL will be
    what this engine uses. 

[^6]: https://en.m.wikipedia.org/wiki/Data-oriented_design

## Basic design ideas

How could an engine focus on enabling open world sandbox RPGs and immersive
sims?

Well, one of the things that truly makes *Bethesda's* games special is their
engine, NetImmerse / Gamebryo / Creation Engine. Despite the bad rap it gets ---
all of it largely undeserved, since it is not really any older than any other
mainstream engine, such as Unreal, has been used to make many other perfectly
good games outside of Bethesda, and is not really more fundamentally flawed than
other engines are (compare with Unreal struggling to take advantage of modern
multicore processors because of its outdated parallel processing architecture)
--- it is largely what is responsible for the unique gameplay character and feel
of their games: far more simulation heavy and dynamic than other games.
Likewise, its specific design approach is probably partly responsible for
Bethesda's ability to put as much sheer content in their games as they do, and
certainly responsible for the singular moddability of their games. There are
several lessons to be learned from Gamebryo, which I will explore more below.

The lesson to learn from immersive sims, on the other hand, are more focused
around design requirements, not specific technical aspects, but one does
directly lead to another. To enable immersive sim style gameplay as much as
possible, game object behavior and properties need to be extremely modular and
encapsulated, to avoid overwhelming complexity, dynamically composable, to allow
for great runtime freedom, and capable of influencing other properties and
especially behavior *without explicit coupling*, so unforseen combinations can
occur. Luckily, there are systems used in the *System Shock*, *Thief*, and
*Caves of Qud* games that can help us with this. For more see below.

## Deeper exploration of design principls

### Data-driven design

The most unique thing about the Gamebryo engine is that it has an extremely
data-driven[^1] design. Essentially this means that all of the game-specific
logic and content, from the game world to the game objects, NPCs, quest systems,
game win and loss states, game mechanics, actor behavior, and more, are
specified using plain old data and a simple scripting language, which the game
engine then picks up and runs much in the same way VLC plays a movie, instead of
much of the game-specific content having to be written in the game engine's
language itself and statically linked to the engine or compiled into the game
engine's executable to produce the final game. This data driven design is even
reflected in the final game output: if you look at the install folder of any
Bethesda game, you will see an engine executable, but you'll also see that all
of the game's scripts and content are separate from that executable. Not plain
text anymore, mind you, it's all been compiled and compressed, but still
definitely separate, which is why you can easily add *new* scripts for the
engine to load! This is a difference of degree, not kind, from modern general
purpose engines like Unreal and Unity, and even some modern big budget studio
engines like REDEngine, but it is a very large degree of difference nonetheless.
There is a reason Bethesda's own first-party DLC content for their games takes
the exact same form that large community content mods do, which cannot be said
for any other engine.

This data-driven design has several benefits. First, it makes adding content to
much less time consuming, simpler, and easier to iterate on, making large,
content-rich, interconnected worlds more feasible. Second, by making a system by
which one can easily and declaratively add new objects and even mechanics to the
game, a scripting system with total access to the full power of the engine's
capabilities --- since that's the main way games will be programmed --- and a
structure that allows adding new scripts and removing old ones even after the
game has been "compiled," this data-driven architecture enables a much more
vibrant and empowered modding community, since first-party content and mods are
essentially on a level playing field: they're both doing the same thing, meaning
mods have the full power game developers also have access to.

A highly data-driven architecture also has the further (theoretical) benefit
that if sufficient compatability is maintained between the data formats old and
new versions of the engine accept, old games can be upgraded with new versions
of the engine at any time with minimal need to actually edit the game!

The Embryo game engine aims to take this much, *much* further, by ensuring that
all the data files used by the engine are in readily readable/writable open
standards, instead of odd proprietary or in-house formats, and that even more of
the game-specific content and behavior can be separated out from the engine,
instead of needing a custom build of the engine or needing to be compiled into
the engine for the final build.

In essence, Embryo Engine wants to be the Emacs of the game engine world. Just
like Emacs is a relatively small core of optimized C code that mostly acts as a
specialized yet powerful interface and rendering system and interpreter for a
highly specialized scripting language, and most of the actual editor has been
made with that scripting language, Embryo wants to act as an optimized,
specialized, but extremely powerful core, on top of which most content will be
built using its scripting language. Likewise, that language should have full
access to the entirity of the engine, to insert itself wherever it pleases and
access all of the engine's capabilities and power, instead of having a limited
set of interface APIs for predefined tasks that the game developers figured mods
should be able to do, just like Emacs Lisp is a far more powerful plugin system
than the limited scripting API provided by NeoVim.

Here are the formats that Embryo will probably use:

- for configuration files, initial game world specifications, game object
  (including actor) specifications, materials, and more, TOML files will be used
- for compiled game world files, game world chunks, and save files, MessagePack
  will be used
- for textures, simple image formats will be used
- for heightmaps, BMPs will be used (in a specific way)
- for 3d objects and parts of scenes, glTF 2.0 will be used

This way, anyone will be easily able to create or modify game files without any
specific suite of tools.

In essence, 

### Simulation-heavy and object-flexible

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
might want to try: immersive sims. From *System Shock 2* to *Deus Ex* to
*Thief*, the key appeal of immersive sims is that you are given a large amount
of powerful tools, and set loose on a problem in an interesting, endlessly
responsive environment, to solve it however you like.

It may not seem obvious how to create a game engine that enables and accelerates
the development of such simulation-heavy games, besides perhaps mandating that
every game object have a mesh and a rigid body (a bad idea), but in watching
gameplay from Deus Ex and Pray, as well as simulation-focused roguelikes like
Dwarf Fortress and Caves of Qud[^2], a few things become clear. First of all,
you need to be able to dynamically add and remove properties and behaviors from
objects in combinations and at times that are not predictable ahead of time, and
that to manage the complexity of such an endeavor, behavior and properties must
be decoupled, encapsulated, and composable; second of all, that those behaviors,
as well as individual game objects and actors, need to be able to effect each
other's behavior without prior expectation of being able to do so; and finally,
that specifying these packages of behavior and properties be as composible and
declarative as possible. Let's look at each of these in turn.

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
  as needed, where components represent compsible and encapsulated units of game
  object properties and behavior is desired (and behavior itself is separated).

- One of the problems with the design of a classic entity component system,
  however, is that since all object behavior is defined in terms of systems,
  which loop through all the entities with the necessary properties to have a
  behavior and perform that behavior for each entity, communication between
  behaviors or systems is difficult, and all combinatorial behavior must be
  specified up front: if I want a new behavior to emerge when an entity has two
  components at the same time, I have to either program that behavior into one
  of the existing systems for those components, or create a new system that
  operates only on entities with both components. Thus I must be able to predict
  and architect all possible combinations of system behavior. This is especially
  true as a result of the fact that it is difficult for systems to pass
  per-entity temporary information --- events and messages --- to other systems
  in an architecturally clean and encapsulated fashion, and difficult for them
  to manipulate the behavior of other systems from afar with those messages
  without tight coupling between them, because systems work on the basis of
  regular behaviors, not event-based ones. Many architectural questions pop up
  when trying to figure this out. Which system handles interlocking behavior?
  How does one system modify the information another acts on, without modifying
  the entity itself? Does the system generating the information modify the
  entities the information is directed at? That produces tight coupling and
  requires more foreknowledge about possible combinations. Furthermore, with a
  classic system-oriented structure, parallel processing becomes more difficult:
  if you have a series of systems that need to run on a group of entities in
  order, you have to run the first system on all the entities, then the next,
  and so on. However, the system might finish the step for some of those
  entities sooner than others, in which case it would be desirable for those
  entities which finished early to be able to move on from that step and work on
  other steps while the late ones are on the last one. This is impossible in a
  classic ECS system, which essentially requires a scatter/gather structure with
  a sync point between each behavior, with at best a few unrelated systems
  running at the same time. This is precisely the sort of design that Unreal had
  that causes it to struggle to make use of modern multicore CPUs. Likewise,
  passing information back and forth between threads in a classic ECS structure
  is difficult. Thus I borrowed a concept from Erlang: message-passing and
  Actor-oriented programming[^3], and a system from Naughty Dog called
  fibers[^4]. In this model, there is a pool of operating system threads, one
  per core, and jobs (or "fibers") that thread through one complete pipeline
  task that needs to be performed in order are generated for any processing that
  needs to be done that can be done in parallel, including actor behavior, and
  pushed onto a job stack which the threads then pull from whenever they're
  finished any previous jobs and begin working on. Specifically, in this case,
  all the behavioral processing necessary for each game object and actor is
  represented as a linear pipeline of transformations to the entity which can be
  performed independently of any of the other entities, just based on last
  frame's game state (as this process is producing this next frame's game
  state). Thus, each actor can update as fast as it can, proceeding through the
  pipeline. (This is actually related to how modern programmable pipelined GPUs
  work.) When they need to communicate, they send messages to either a global
  queue by message type, which each actor can subscribe to (such global messages
  are called "events") or they send that message to an actor's unique queue.
  General events in the world, such as collisons, entities entering trigger
  areas, update ticks, user input, will be distributed to all actors via this
  messaging and event system as well, meaning that *all actor behavior is
  triggered by events/messages*, including stuff that happens on each frame. How
  this works is inspired by the architecture of Caves of Qud's message based
  ECS, where events are fed to the first behavior on an object (according to a
  priority system), and that behavior can choose to act on any, all, or none of
  those events, and then *modify those events* or produce new ones (either to
  pass on, or to send to other entities, or to notify the game state that
  something at a higher level needs to change), and this new set of events is
  then passed to the next behavior. Thus, behaviors can modify each other by
  modifying the events they receive, since all behavior is described by and
  triggered by events, without any coupling whatsovever. I highly recommend you
  watch the talk in the footnotes if you want to learn more about this.
  
- As for our third and final point, easy game object and behavior/property
  specification and assembly in a declarative manner, that is already covered by
  having a data-driven design.

### Knowing where to draw the line

Another principle of design I want to discuss here is less derived from
immersive sims and the Gamebryo engine, and more one derived from necessity: in
an attempt to limit the scope and scale of this endeavor to something relatively
more reasonable, at least at the outset, I've set a specific era of games in
mind that I want my engine's graphical, animation, and similar capabilities to
be able to match, and beyond that, I'm not going to worry about it, besides
making the engine extensible so that it's a good platform for doing more
advanced things if people want to. Everything is a tradeoff between benefit and
complexity, and for my limitations as an individual programmer, I've found that
the graphics algorithms and similar capabilities of games from around 1998-2005
or so seem to be at the sweet spot of that tradeoff for me: any increase in
capability increases complexity at a vastly disproportional rate compared to the
actual tangible gains received in the meaningful artistic expressiveness of the
game engine (not just shininess), especially given that the sheer power of
modern hardware removes the real biggest limitations on the scope and
expressivity of games of those eras, whereas any decrease in capability
diminishes the complexity of this endeavor only slightly, while walling off
large portions of game expressiveness --- the types of games and visuals that
can be made with the engine's technology.

Of course, I'm not blindly aiming for one level of technology. I'm using modern
software architecture, data structures, and algorithms to make the engine
performant and flexible and capable of taking advantage of modern hardware. i'm
using modern data-oriented design, modern graphics programming techniques and
APIs, and modern object model and parallel processing models. Thus, although it
may look old on the surface, this engine will have fresh, shiny, and well
thought through, modern internals. In effect I'm doing what pixel art indie devs
have been doing for quite awhile now, keeping the scope and difficulty of
developing their game's graphics and content limited by picking an old style of
game to make with modern tools, so you get the productivity of modern tools
brought to bear on simpler problems --- I'm just doing that for the polygon era,
not the pixelated era!

Of course, this means it probably isn't for everybody, or even anybody, but,
well. I'm making the program ;)

### Knowing specifically what you want and optimizing ruthlessly for that

In my opinion, carefully choosing data structures, algorithms, and overall
architectures that are best suited to the specific use cases you have for them,
and are acceptable in the general case, yet are as intuitive and straightforward
as possible, will always win out over choosing the latest and greatest cutting
edge algorithm from the newest research paper that supposedly has the best
possible general case performance for every single thing, and trying to deal
with the resultant complexity and unintuitiveness. This is because algorithms
that are good in one specific case and fine in the general case are always going
to be better in that specific case then algorithms that are trying to be good in
all cases, and they are also always going to be easier to maintain because they
are simpler and usually more intuitive, so if you know what kind of game you are
going to make ahead of time, then you can outpace people doing more advanced
things while also keeping your code simpler. It's a matter of slow and steady
wins the race, in all honesty. I have a few small case studies in this that I
will use to demonstrate how I apply this philosophy to my game engine.

First is my choice to use sparse set ECS instead of archetype based ECS[^7]. If
you look at almost any major entity component system framework around today,
such as flecs, Bevy, or Amethyst, you'll quickly notice that they are all using
the exact same model for storing the components in their entity component system
architectures: archetypes. Archetypes are typically used because they combine
all entities with the same component signatures (and their associated
components) in one place, so the systems that iterate over certain types of
entities will be able to more quickly iterate through those entities because
entities will be already grouped by type and have all of their components packed
into their spot in that group already. This is an intuitive way to attempt to
maximize the *data oriented design based* benefits of ECS --- since everything
is done through batch processing via systems, you ensure that your systems can
execute as fast as possible in three ways:

1. because only entities with a valid set of components for your system's query
   or ever even iterated on in the first place, so there are no wasted
   iterations where you have to discard a nonmatching entity,
2. similarly to above, because there isn't *branching* based on whether an
   entity is a valid candidate for that system to process, for the same reason
   as above, and
3. because the amount of the data that your system is processing that can fit
   into your processor's cache at once is maximized, because all of the data is
   packed as tightly as possible.

In essence, archetype-based ECS is primarily using entity component systems as a
data structure that is used to do data oriented design style optimizations, and
it is attempting to optimize for the general case --- the most common types of
things you do with entity components systems.

So, why did I choose to go with a sparse set ECS instead? For two reasons:

1. if implemented correctly, sparse sets are not significantly slower than
   archetypes for iteration or selecting entities with various components, and
   thanks to EnTT-style groups there are in fact optimizations that you can do
   for sparse ECS systems that can bring them up to parity or even superiority
   to archetype-based ECS in that area, if needed, but compared to
   archetype-based ECS systems sparse set based ECS systems are trivial to
   implement correctly and understand, for nearly equivalent performance anyway,
   and
2. crucially, **archetype-based ECS are much slower when adding or removing
   components from entities**, since when the signature of an entity changes,
   that entire entity including all of its components must be removed from one
   archetype pool, new memory must be allocated in another archetype pool, and
   then the entity must be inserted there. This in essence defeats the *entire
   purpose* of ECS as a game object architecture with unique benefits, by making
   the one truly unique thing ECSs can do that isn't just an under the hood
   optimization *the most expensive possible operation*, more expensive even
   than anything equivalent in non-ECS systems, since it requires essentially
   copying your object and reallocating memory. So it is all well and good if
   you are using entity component systems primarily as a data acceleration
   structure, in order to enable better data oriented design, but it is less
   good if you want to use entity components because of the benefits specific to
   ECS, such as being able to dynamically change the properties and behavior of
   objects at runtime in a composable and encapsulated manner, or the ability to
   use your ECS as a sort of intelligent database and query system over your
   game world that can be used for dynamic and ephemeral properties using things
   like tag components. So you see, while archetype-based ECS optimizes for the
   general case, in doing so it actually undercuts much of the unique benefits
   of ECS in the first place, reducing it to just an acceleration data
   structure.

Note: what I'm saying about the relative performance characteristics of sparse
set and archetype-based ECS systems isn't just based on a theoretical
understanding of the nature of their algorithms either. It bears itself out in
objective testing via benchmarks as well:
<https://github.com/abeimler/ecs_benchmark>

Another case study would be my choice of spatial partitioning algorithms for
location and distence queries, broad phase collision detection, and frustum
culling, which are all important for large worlds and performant intelligent AI.
The new hotness right now according to many people and several textbooks that I
have read on the subject is bounding volume hierarchies, because due to the fact
that they are built from the objects themselves instead of subdividing the space
the objects are in and then sorting the objects into that space, they can
require less work and do more accurate narrowing since the subdivisions fit more
tightly around specific objects, so you don't for instance get fairly widely
separated objects in the same node just because they're within a certain radius,
and you don't get small objects way too high in your partitioning hierarchy just
because they happen to fall on a bunch of spatial partition boundaries. So they
are to some degree faster in a general sense. the issue is that they are in fact
less suitable for situations where you have a lot of dynamically moving objects,
since the hierarchy is itself built from the objects and so will likely need to
be completely rebuilt anytime an objects moves or risk degrading over time until
its performance is terrible if you use an insertion method. Despite this, in
recent years people have begun recommending then even for dynamic objects for
some reason. On the other hand, traditional spatial partitioning is much better
than bounding volume hierarchies for dynamically moving objects, because since
it is space itself that you are partitioning, when an object moves you rarely
need to totally rebuild the hierarchy. At most you will need to split a node
here or there into multiple leaf nodes, and in the majority of cases you will
just remove the moving object from one node and put it in another. So, since I'm
frankly not too concerned with having a best case broad phase algorithm for
collisions with static meshes, since in any given world chunk there will
probably only be a few large static meshes on the scene, and any complexities to
their geometry have will have to be dealt with in the narrow phase for collision
resolution anyway, which isn't even going to be handled by my engine but instead
by a physics middleware, a slightly non-optimal solution for static meshes is
probably acceptable. Meanwhile, I *will* have a large amount of dynamically
moving entities in most world chunks, so I instead chose to go with spatial
partitioning. The traditional spatial partitioning algorithm for 3D spaces is
the octree, but given the fact that 90% of the kind of games that I want my
engine to enable will take place in a setting that has some form or degree of
gravity and is more horizontally oriented than vertically oriented, I decided
that even those were overkill and went for a quadtree.

A related choice I made is in the fundamental architecture of my engine: it has
a separate render and update thread that are not synchronized whatsoever, so the
render thread just runs on whatever game state it has as fast as the sync rate
lets it (or as fast as it can if frame rate capping has been turned off), and
the update thread runs as fast as its update interval has been set to, with the
render thread just working on rendering the last game state it had access to,
and the update thread sending over the relevant portions of the new game state
whenever it produces a new one that needs to be rendered, which is then queued
by the render thread for it to get to as soon as it can. This is similar to how
the Destiny rendering engine communicates with its update threads.[^11] It goes
even further than this too, because on the update thread parallelism is employed
even more thoroughly: thanks to rayon, on my game has a pool of threads equal in
size to the number of cores on the CPU, not including the render an update
thread, and so almost every task on the update thread is done by creating jobs
on the job queue for the thread pool to take care of, whether that is loading
and processing resources, or handling each iteration of various systems, or even
running multiple systems at the same time if their data requirements allow that.
That's all pretty par for the course for Rust ECS systems though --- my more
interesting choice was to actually pipeline all scripted systems. The way other
Rust ECS systems do their parallel processing still employs the scatter-gather
methodology, where each sequential system farms out all of its work as a bunch
of jobs on the thread pool and then joins on those jobs and blocks until they
all finish, and only after that system is done blocking and all of those jobs
have finished can work proceed to any of the systems that depend on that one.
Some of them also attempt to parallelize some systems, so if you have two
systems that access a disjoint set of components, they can run at the same time.
But this is barely the synchronous function parallel model[^8], which is
actually ironically what has been holding Unreal Engine back from efficiently
using CPUs in the modern day.[^9] This still has some inherent problems, so
instead I tried to bring in some of the insights that the Destiny[^10][^11] and
Naughty Dog GDC talks provided, by instead transforming all of the scripted
systems for entities in the game engine into per entity "threads" (in the long
winding string sense) through the pipeline of those systems, where the thread
only goes through the single iteration of each system relevant to the given
entity. Essentially, instead of it working on the basis of rows, it works on the
basis of columns. This means that entities can proceed through the entire
pipeline of systems relevant to them without having to wait for other entities
to catch up at the boundary between each sequential system, so that each entity
in the game can have radically different systems running on them that may not
match in their place in the pipeline (for example, one entity may have two
systems in between the next system that it has in common with another entity,
thereby forcing the second entity to wait before it can process its next system
behavior), or can have systems where each iteration may take a radically
different amount of time, and things won't get slowed down. Of course this is a
trade-off, because it gives up a lot of the data oriented design benefits of
doing that processing using systems, which is why all of the built-in engine
systems will actually use traditional scatter/gather batch processing
methodology, since the execution time of each individual iteration is unlikely
to vary too much or to be particularly long and there are unlikely to be very
long chains of built-in systems that entities have to go through that are
different between different entities, so having to wait for each other to catch
up at the boundary of each dependent sequential system is unlikely to be a
problem. But for scripted systems, which are likely to be small and numerous and
highly variable between individual entities, and are likely to also be much
slower than rust code and more variable in their execution time, I think this
was a trade-off that was crucial to make. Importantly, the reason I spent so
much time on parallel processing is that modern CPUs are much more powerful in
terms of parallel processing then they are in terms of single core straight line
speed, and that Divergence is likely only to increase, so optimizing for
parallel processing is a core and fundamental way to give myself Headroom with
single course sequential processing operations, so I don't have to be cycle
counting as much. At the same time, I used an approach similar to Naughty Dog's,
where almost all tasks are done as jobs on thread pools, but sequential tasks
are made sequential by being sequentially launched and joined in other threads,
and *didn't* attempt a full directed acyclic graph of individual tasks and
dependencies so that I could run the entire engine as jobs, like Destiny did,
because that wasn't necessary for my use case.

[^1]: See this talk to understand what data-driven means for engines:
    <https://gdcvault.com/play/1022543/A-Data-Driven-Object>.

[^2]: This talk is an especially good summary of how data-driven design, a
    flexible entity-component system, and a message-passing based event system
    that bubbles events up through the components on each entity much like
    events bubble up through DOM elements, is an especially good one for
    understanding what I'm talking about here:
    <https://www.youtube.com/watch?v=U03XXzcThGU>

[^3]: https://en.wikipedia.org/wiki/Actor_model

[^4]: https://www.gdcvault.com/play/1022186/Parallelizing-the-Naughty-Dog-Engine

[^6]: https://en.m.wikipedia.org/wiki/Data-oriented_design

[^7]: For a basic intro about what these are, see the excellent *ECS back and
    forth* series, part 2: https://skypjack.github.io/2019-03-07-ecs-baf-part-2/

[^8]:
    <https://www.gamedeveloper.com/programming/multithreaded-game-engine-architectures>.
    See also: <https://vkguide.dev/docs/extra-chapter/multithreading/>

[^9]: https://www.youtube.com/watch?v=-x0orqFrHGA

[^10]: https://www.youtube.com/watch?v=v2Q_zHG3vqg

[^11]: https://www.gdcvault.com/play/1021926/Destiny-s-Multithreaded-Rendering


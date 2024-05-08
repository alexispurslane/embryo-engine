use super::{Component, Entity};

pub struct HierarchyComponent {
    pub parent: Entity,
    pub depth: usize,
}

impl Component for HierarchyComponent {
    fn get_id() -> super::ComponentID {
        "HierarchyComponent"
    }
    fn add_hook(
        &mut self,
        current_entity: Entity,
        game_state: &mut crate::update_thread::GameState,
    ) {
        let parent_depth = game_state
            .entities
            .get_component::<HierarchyComponent>(self.parent)
            .map_or(0, |p| p.depth);
        self.depth = parent_depth + 1;
    }
}

impl HierarchyComponent {
    pub fn new(parent: Entity) -> Self {
        Self { parent, depth: 0 }
    }
}

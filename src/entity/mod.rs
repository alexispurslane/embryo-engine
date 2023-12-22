/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
};

use crate::systems;

use self::mesh_component::ModelComponent;

pub mod camera_component;
pub mod light_component;
pub mod mesh_component;
pub mod terrain_component;
pub mod transform_component;

pub type ComponentID = &'static str;
pub type EntityID = usize;

pub trait Component {
    fn get_id() -> ComponentID;
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityID,
    pub generation: usize,
}

pub trait ComponentVec {
    fn add_new_entity_col(&mut self);
    fn remove_entity_col(&mut self, eid: EntityID);

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

type ComponentVecConcrete<T> = RefCell<Vec<Option<T>>>;
impl<T: Component + 'static> ComponentVec for ComponentVecConcrete<T> {
    fn add_new_entity_col(&mut self) {
        self.get_mut().push(None);
    }
    fn remove_entity_col(&mut self, eid: EntityID) {
        self.get_mut()[eid] = None;
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

pub struct EntitySystem {
    pub entity_count: EntityID,
    pub current_generation: usize,
    pub current_entity_generations: HashMap<EntityID, usize>,
    pub free_entities: Vec<EntityID>,
    pub components: HashMap<ComponentID, Box<dyn ComponentVec>>,
}

impl EntitySystem {
    pub fn new() -> Self {
        Self {
            current_generation: 0,
            entity_count: 0,
            components: HashMap::new(),
            current_entity_generations: HashMap::new(),
            free_entities: vec![],
        }
    }

    pub fn gen_entity(&mut self) -> Entity {
        let e = if let Some(eid) = self.free_entities.pop() {
            Entity {
                id: eid,
                generation: self.current_generation,
            }
        } else {
            // New entity handle
            self.entity_count += 1;

            // Add all the entity's components to the registry
            for (_cid, component_list) in self.components.iter_mut() {
                component_list.add_new_entity_col();
            }

            Entity {
                id: self.entity_count - 1,
                generation: self.current_generation,
            }
        };
        self.current_entity_generations.insert(e.id, e.generation);
        e
    }

    pub fn delete_entity(&mut self, entity: Entity) {
        if entity.generation != self.current_entity_generations[&entity.id] {
            println!("WARNING: Tried to use recycled entity ID to refer to old entity");
            return;
        }
        self.current_generation += 1;
        for (_cid, component_list) in self.components.iter_mut() {
            component_list.remove_entity_col(entity.id);
        }
        self.free_entities.push(entity.id);
    }

    pub fn add_component<T: Component + 'static>(&mut self, entity: Entity, c: T) {
        if entity.generation != self.current_entity_generations[&entity.id] {
            println!("WARNING: Tried to use recycled entity ID to refer to old entity");
            return;
        }

        if let Some(component_vec) = self
            .components
            .get_mut(&T::get_id())
            .and_then(|x| x.as_any_mut().downcast_mut::<ComponentVecConcrete<T>>())
        {
            component_vec.get_mut()[entity.id] = Some(c);
        } else {
            let mut h: Vec<Option<T>> = Vec::new();
            h.resize_with(self.entity_count, || None);

            h[entity.id] = Some(c);
            self.components
                .insert(T::get_id(), Box::new(RefCell::new(h)));
        }
    }

    // Returns true if an asset unload cycle is needed after deleting this component
    pub fn remove_component<T: Component + 'static>(&mut self, entity: Entity) -> bool {
        if entity.generation != self.current_entity_generations[&entity.id] {
            println!("WARNING: Tried to use recycled entity ID to refer to old entity");
            return false;
        }

        if let Some(component_vec) = self.components.get_mut(&T::get_id()) {
            component_vec.remove_entity_col(entity.id);
        }
        T::get_id() == ModelComponent::get_id()
    }

    pub fn get_component<T: Component + 'static>(&self, entity: Entity) -> Option<Ref<T>> {
        if entity.generation != self.current_entity_generations[&entity.id] {
            println!("WARNING: Tried to use recycled entity ID to refer to old entity in a situation where a result is required");
            return None;
        }

        let val = Ref::map(self.get_component_vec::<T>(), |vec: &Vec<Option<T>>| {
            &vec[entity.id]
        });
        if val.is_some() {
            Some(Ref::map(val, |x| x.as_ref().unwrap()))
        } else {
            None
        }
    }

    pub fn get_component_mut<T: Component + 'static>(&self, entity: Entity) -> Option<RefMut<T>> {
        if entity.generation != self.current_entity_generations[&entity.id] {
            println!("WARNING: Tried to use recycled entity ID to refer to old entity in a situation where a result is required");
            return None;
        }

        let val = RefMut::map(
            self.get_component_vec_mut::<T>(),
            |vec: &mut Vec<Option<T>>| &mut vec[entity.id],
        );
        if val.is_some() {
            Some(RefMut::map(val, |x| x.as_mut().unwrap()))
        } else {
            None
        }
    }

    pub fn get_current_entity_from_id(&self, eid: EntityID) -> Option<Entity> {
        if !self.free_entities.contains(&eid) {
            self.current_entity_generations.get(&eid).map(|gen| Entity {
                id: eid,
                generation: *gen,
            })
        } else {
            None
        }
    }

    pub fn get_component_vec<T: Component + 'static>(&self) -> Ref<Vec<Option<T>>> {
        self.components
            .get(T::get_id())
            .map(|x| {
                x.as_any()
                    .downcast_ref::<ComponentVecConcrete<T>>()
                    .expect("Incorrect downcast of component vector to type!")
                    .borrow()
            })
            .expect(
                format!(
                    "Tried to get nonexistant component vector {:?}",
                    T::get_id()
                )
                .as_str(),
            )
    }

    pub fn get_component_vec_mut<T: Component + 'static>(&self) -> RefMut<Vec<Option<T>>> {
        self.components
            .get(T::get_id())
            .map(|x| {
                x.as_any()
                    .downcast_ref::<ComponentVecConcrete<T>>()
                    .expect("Incorrect downcast of component vector to type!")
                    .borrow_mut()
            })
            .expect(
                format!(
                    "Tried to get nonexistant component vector {:?}",
                    T::get_id()
                )
                .as_str(),
            )
    }

    pub fn get_with_component<'a, T: Component + 'static>(
        &'a self,
        ts: &'a Ref<Vec<Option<T>>>,
    ) -> impl Iterator<Item = (EntityID, &T)> {
        ts.iter()
            .enumerate()
            .filter_map(|(i, mc)| Some((i, mc.as_ref()?)))
    }

    // Lifetimes mean that self has to live at least as long as ts and us, I
    // think? Which is fine, since ts and us are *drawn from self*
    pub fn get_with_components<'a, T: Component + 'static, U: Component + 'static>(
        &'a self,
        ts: &'a Ref<Vec<Option<T>>>,
        us: &'a Ref<Vec<Option<U>>>,
    ) -> impl Iterator<Item = (EntityID, &T, &U)> {
        ts.iter()
            .enumerate()
            .zip(us.iter())
            .filter_map(|((i, t), u)| Some((i, t.as_ref()?, u.as_ref()?)))
    }

    pub fn get_with_components_mut<'a, T: Component + 'static, U: Component + 'static>(
        &'a self,
        ts: &'a mut RefMut<Vec<Option<T>>>,
        us: &'a mut RefMut<Vec<Option<U>>>,
    ) -> impl Iterator<Item = (EntityID, &mut T, &mut U)> {
        ts.iter_mut()
            .enumerate()
            .zip(us.iter_mut())
            .filter_map(|((i, t), u)| Some((i, t.as_mut()?, u.as_mut()?)))
    }
}

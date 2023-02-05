use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
};

pub mod camera_component;
pub mod mesh_component;
pub mod transform_component;

pub type ComponentID = &'static str;
pub type EntityID = usize;

pub trait Component {
    fn get_id() -> ComponentID;
}

pub struct Entity {
    pub id: EntityID,
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
    pub components: HashMap<ComponentID, Box<dyn ComponentVec>>,
}

impl EntitySystem {
    pub fn new() -> Self {
        Self {
            entity_count: 0,
            components: HashMap::new(),
        }
    }

    pub fn new_entity(&mut self) -> Entity {
        // New entity handle
        let e = Entity {
            id: self.entity_count,
        };
        self.entity_count += 1;

        // Add all the entity's components to the registry
        for (_cid, component_list) in self.components.iter_mut() {
            component_list.add_new_entity_col();
        }
        e
    }

    pub fn delete_entity(&mut self, eid: EntityID) {
        for (_cid, component_list) in self.components.iter_mut() {
            component_list.remove_entity_col(eid);
        }
    }

    pub fn add_component<T: Component + 'static>(&mut self, eid: EntityID, c: T) {
        if let Some(component_vec) = self
            .components
            .get_mut(&T::get_id())
            .and_then(|x| x.as_any_mut().downcast_mut::<ComponentVecConcrete<T>>())
        {
            component_vec.get_mut()[eid] = Some(c);
        } else {
            let mut h: Vec<Option<T>> = Vec::new();
            h.resize_with(self.entity_count, || None);

            h[eid] = Some(c);
            self.components
                .insert(T::get_id(), Box::new(RefCell::new(h)));
        }
    }

    pub fn remove_component<T: Component + 'static>(&mut self, eid: EntityID) {
        if let Some(component_vec) = self.components.get_mut(&T::get_id()) {
            component_vec.remove_entity_col(eid);
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

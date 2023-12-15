use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender},
    thread::{self, JoinHandle},
};

use gl::Gl;

use rayon::prelude::*;
use ritelinked::LinkedHashSet;

use crate::entity::{
    mesh_component::{Model, ModelComponent},
    Entity, EntitySystem,
};

// TODO: Actually use this.
pub enum ResourceRequest {
    Models(Vec<Entity>),
    Textures(Vec<String>),
}

pub struct ResourceManager {
    pub response_sender: Sender<(String, Model)>,
    pub response_receiver: Receiver<(String, Model)>,

    pub models: HashMap<String, Model>,
}

impl ResourceManager {
    pub fn new() -> Self {
        let (ress, resr) = channel();

        Self {
            response_sender: ress,
            response_receiver: resr,

            models: HashMap::new(),
        }
    }

    // Although this will technically work if you do a bunch of little calls, to
    // make it avoid double-loading models you'd have to make sure each call is
    // actually sequential, since if you call with only one entity then the only
    // unique-ing that can occur is only based on the models that have already
    // completed loading entirely, so you might end up loading a model that's
    // unique to the models that have finished loading but duplicates the work
    // of a model that's currently in the process of loading. Whereas, if you
    // call this with a good size batch, then it knows a good section of what
    // other models will be loading at the same time as each model you specify
    // --- the other models in the batch! --- and so it can unique between them
    // and save you time and memory while still doing stuff in parallel, without
    // having to wait for everything to finish.
    pub fn request_model_batch(&mut self, entities: &EntitySystem, new_entities: &Vec<Entity>) {
        // Filter out all the entities that don't actually need to be loaded at all
        let new_models = new_entities
            .iter()
            .map(|entity| {
                (
                    entities
                        .get_component::<ModelComponent>(*entity)
                        .as_ref()
                        .expect("Tried to load a model for an entity with no model!")
                        .path
                        .clone(),
                    *entity,
                )
            })
            .filter(|(path, entity)| {
                // If this entity's model is already loaded, just add it to that
                // model's instance list and then filter it out
                if let Some(entry) = self.models.get_mut(path) {
                    entry.entities.insert(*entity);
                    entry.entities_dirty_flag = true;
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        let resp = self.response_sender.clone();
        Self::load_models(resp, new_models);
    }

    /// Checks to see if there's a new batch of models done loading. If there
    /// is, then block and integrate it. Else return. Returns true if there was
    /// new stuff and false otherwise.
    pub fn try_integrate_loaded_models(&mut self, gl: &Gl) -> bool {
        if let Ok((path, mut model)) = self.response_receiver.try_recv() {
            if let Some(og_model) = self.models.get_mut(&path) {
                // A race condition must've happened where the same model was
                // requested in two different batches and one wasn't integrated
                // before the other decided to load that model too, so they both
                // loaded the model. This is suboptimal but all we can do now is
                // recover gracefully.
                og_model.entities.extend(model.entities);
                og_model.entities_dirty_flag = true;
            } else {
                // A proper new model! Just add it to the resources pile
                model.setup_model_gl(gl);
                self.models.insert(path, model);
            }
            true
        } else {
            false
        }
    }

    fn load_models(response_sender: Sender<(String, Model)>, new_entities: Vec<(String, Entity)>) {
        // Figure out which unique models need to be loaded for this batch of
        // entities, and which entities use them
        let models_requested = new_entities.iter().fold(
            HashMap::new(),
            |mut registry: HashMap<String, LinkedHashSet<Entity>>,
             (path, entity): &(String, Entity)| {
                if let Some(entry) = registry.get_mut(path) {
                    entry.insert(*entity);
                } else {
                    let mut hs = LinkedHashSet::new();
                    hs.insert(*entity);
                    registry.insert(path.clone(), hs);
                }
                registry
            },
        );

        // Load the models
        models_requested.into_iter().for_each(|(path, entities)| {
            let response_sender = response_sender.clone();
            rayon::spawn(move || {
                let time = std::time::Instant::now();
                let start_gltf_time = time.elapsed().as_millis();
                let gltf = gltf::import(path.clone()).expect(&format!(
                    "Unable to interpret model file {} as glTF 2.0 file.",
                    path
                ));
                let end_gltf_time = time.elapsed().as_millis();
                println!(
                    "GLTF loaded for {} in time {}ms",
                    path,
                    end_gltf_time - start_gltf_time
                );

                let start_process_time = time.elapsed().as_millis();
                let mut model = Model::from_gltf(gltf).expect("Unable to load model");
                model.entities.extend(entities);
                let end_process_time = time.elapsed().as_millis();
                println!(
                    "GLTF processed to native formats for {} in time {}ms",
                    path,
                    end_process_time - start_process_time
                );

                let _ = response_sender.send((path.to_string(), model)).unwrap();
            })
        });
    }
}

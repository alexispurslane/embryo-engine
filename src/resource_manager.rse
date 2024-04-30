/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use gl::Gl;

use rayon::prelude::*;

use crate::entity::{
    mesh_component::{Model, ModelComponent},
    Entity, EntitySystem,
};

#[derive(Debug)]
pub enum ResourceRequest {
    Models(Vec<(String, Entity)>),
    UnloadModels(Vec<(String, Entity)>),
    Textures(Vec<String>),
    WorldChunks(Vec<(u32, u32)>),
}

#[derive(Clone)]
pub struct ResourceManager {
    pub request_sender: Sender<ResourceRequest>,
    pub model_response: Receiver<(String, Model)>,
    pub texture_response: Receiver<(u32, u32, Vec<u8>)>,
    pub chunk_response: Receiver<()>,
    state: Arc<ResourceManagerState>,
}

#[derive(Debug, PartialEq, Eq)]
enum LoadingState {
    Loading,
    Loaded,
}

struct ResourceManagerState {
    loaded_loading_models: RwLock<HashMap<String, (LoadingState, HashSet<Entity>)>>,
    loaded_loading_chunks: RwLock<HashSet<(LoadingState, (u32, u32))>>,
    loaded_loading_texs: RwLock<HashSet<(LoadingState, String)>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        let (reqs, request_receiver) = unbounded();
        let (model_response_sender, model_response) = unbounded();
        let (tex_response_sender, texture_response) = unbounded();
        let (chunk_response_sender, chunk_response) = unbounded();

        let state = Arc::new(ResourceManagerState {
            loaded_loading_models: RwLock::new(HashMap::new()),
            loaded_loading_chunks: RwLock::new(HashSet::new()),
            loaded_loading_texs: RwLock::new(HashSet::new()),
        });
        {
            let state = state.clone();
            thread::Builder::new()
                .name("resource manager".into())
                .spawn(move || {
                    for request in request_receiver.iter() {
                        match request {
                            ResourceRequest::Models(model_reqs) => {
                                let mut loaded_loading_models =
                                    state.loaded_loading_models.write().unwrap();
                                for (path, using_entity) in model_reqs {
                                    if let Some((loading_state, entities)) =
                                        loaded_loading_models.get_mut(&path)
                                    {
                                        // We've already loaded the model
                                        // previously, so document that these
                                        // entities are using it...
                                        entities.insert(using_entity);
                                        if *loading_state == LoadingState::Loaded {
                                            // If the model is already loaded for the
                                            // client, then we need to send a message to
                                            // the client to update its entities list
                                            // for this model based on these new
                                            // entities. If it's loading, though, any
                                            // changes to the entities list in our
                                            // registry will be picked up when it's
                                            // finished loading and integrated, so we
                                            // wouldn't need to do anything.
                                            model_response_sender
                                                .send((path, Model::default()))
                                                .unwrap();
                                        }
                                    } else {
                                        loaded_loading_models.insert(
                                            path.clone(),
                                            (LoadingState::Loading, HashSet::from([using_entity])),
                                        );
                                        Self::spawn_model_loader(
                                            model_response_sender.clone(),
                                            path,
                                        );
                                    }
                                }
                            }
                            ResourceRequest::UnloadModels(model_unload_reqs) => {
                                let mut loaded_loading_models =
                                    state.loaded_loading_models.write().unwrap();
                                for (model, entity) in model_unload_reqs {
                                    if let Some((_, using)) = loaded_loading_models.get_mut(&model)
                                    {
                                        using.remove(&entity);
                                        if using.is_empty() {
                                            loaded_loading_models.remove_entry(&model);
                                        }
                                    }
                                }
                            }
                            ResourceRequest::Textures(texture_reqs) => unimplemented!(),
                            ResourceRequest::WorldChunks(chunk_reqs) => unimplemented!(),
                        }
                    }
                })
                .unwrap();
        }

        Self {
            request_sender: reqs,
            model_response,
            texture_response,
            chunk_response,
            state,
        }
    }

    pub fn request_models(&self, requests: Vec<(String, Entity)>) {
        self.request_sender
            .send(ResourceRequest::Models(requests))
            .unwrap()
    }
    pub fn request_unload_models(&self, requests: Vec<(String, Entity)>) {
        self.request_sender
            .send(ResourceRequest::UnloadModels(requests))
            .unwrap()
    }

    /// Checks to see if there's a new batch of models done loading. If there
    /// is, then block and integrate it. Else return. Returns true if there was
    /// new stuff and false otherwise.
    ///
    /// FIXME: remove models not in the loaded model's entity list from the og
    /// model's entity list as well, so we can unload models properly
    pub fn try_integrate_loaded_models(
        &self,
        models: &mut HashMap<String, Model>,
        gl: &Gl,
    ) -> bool {
        if let Ok((path, mut model)) = self.model_response.try_recv() {
            let mut loaded_loading_models = self.state.loaded_loading_models.write().unwrap();
            let (state, entities) = loaded_loading_models.get_mut(&path).unwrap();
            if let Some(og_model) = models.get_mut(&path) {
                og_model.entities.extend(entities.iter());
                og_model.entities_dirty_flag = true;
            } else {
                if model.meshes.is_empty() {
                    panic!("Received empty model in real model add branch. This means a model that shows as previously loaded for resource manager is missing from client registry, this is impossible to recover from!");
                }

                *state = LoadingState::Loaded;

                model.setup_model_gl(gl);
                model.entities.extend(entities.iter());

                models.insert(path, model);
            }
            true
        } else {
            false
        }
    }

    fn spawn_model_loader(model_response_sender: Sender<(String, Model)>, path: String) {
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
            let end_process_time = time.elapsed().as_millis();
            println!(
                "GLTF processed to native formats for {} in time {}ms",
                path,
                end_process_time - start_process_time
            );

            let _ = model_response_sender
                .send((path.to_string(), model))
                .unwrap();
        });
    }
}

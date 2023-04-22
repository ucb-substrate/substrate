//! Preprocessing operations for netlist exporting.

use std::collections::{HashSet, VecDeque};

use slotmap::SecondaryMap;

use super::super::circuit::Reference;
use super::super::context::{ModuleKey, SchematicData};
use super::super::module::Module;
use crate::deps::arcstr::ArcStr;
use crate::error::{with_err_context, ErrorContext, Result};
use crate::index::IndexOwned;
use crate::schematic::signal::SignalPathBuf;

/// The state of a nestlist preprocessor.
struct NetlistPreprocessor<'a> {
    data: &'a SchematicData,
    netlist_order: Vec<ModuleKey>,
    queue: VecDeque<ModuleKey>,
    top: ModuleKey,

    modules: SecondaryMap<ModuleKey, Module>,
    visited: SecondaryMap<ModuleKey, bool>,
    mod_names: HashSet<ArcStr>,
}

/// The preprocessed netlist with deduplicated names and top-down ordering.
#[derive(Debug, Clone)]
pub(crate) struct PreprocessedNetlist {
    pub(crate) modules: SecondaryMap<ModuleKey, Module>,
    pub(crate) netlist_order: Vec<ModuleKey>,
    pub(crate) top: ModuleKey,
}

/// Preprocesses the provided netlist.
pub(crate) fn preprocess_netlist(
    data: &SchematicData,
    top: ModuleKey,
) -> Result<PreprocessedNetlist> {
    with_err_context(NetlistPreprocessor::new(data, top).preprocess(), || {
        ErrorContext::Task(arcstr::literal!("preprocessing netlist"))
    })
}

impl<'a> NetlistPreprocessor<'a> {
    /// Creates a new [`NetlistPreprocessor`].
    pub fn new(data: &'a SchematicData, top: ModuleKey) -> Self {
        Self {
            data,
            netlist_order: Vec::new(),
            queue: VecDeque::new(),
            top,
            modules: SecondaryMap::new(),
            visited: SecondaryMap::new(),
            mod_names: HashSet::new(),
        }
    }

    /// Preprocesses the netlist.
    pub fn preprocess(mut self) -> Result<PreprocessedNetlist> {
        self.bfs()?;
        Ok(PreprocessedNetlist {
            modules: self.modules,
            netlist_order: self.netlist_order,
            top: self.top,
        })
    }

    /// Makes a first pass rewriting of the module list.
    ///
    /// Does the following:
    /// 1. Makes a list of all modules that are actually used by the top module
    /// or its submodules.
    /// 2. Rewrites duplicate module names.
    /// 3. Rewrites duplicate instance names within a module.
    fn bfs(&mut self) -> Result<()> {
        self.queue.push_back(self.top);
        self.visited.insert(self.top, true);

        while !self.queue.is_empty() {
            // Cannot panic since we check that the queue is not empty
            let id = self.queue.pop_front().unwrap();
            let module = self.data.get_by_id(id)?;

            for inst in module.instances() {
                if let Reference::Local(module) = inst.module() {
                    let id = module.id;
                    if !self.visited.contains_key(id) {
                        self.visited.insert(id, true);
                        self.queue.push_back(id);
                    }
                }
            }
            let module = self.fix_module_names(module);
            self.modules.insert(id, module);
            self.netlist_order.push(id);
        }

        Ok(())
    }

    /// Fixes duplicate module names.
    fn fix_module_names(&mut self, module: &Module) -> Module {
        let mut module = module.clone();
        self.save_and_rename_module(&mut module);
        self.rename_instances(&mut module);
        module
    }

    /// Renames a module if its current name is already taken,
    /// then adds the updated name to the list of used names ([`Self::mod_names`]).
    fn save_and_rename_module(&mut self, module: &mut Module) {
        if self.mod_names.contains(module.name()) {
            let mut i = 1;
            let name = loop {
                let name = arcstr::format!("{}_{}", module.name(), i);
                if !self.mod_names.contains(&name) {
                    break name;
                }
                i += 1;
            };

            module.set_name(name);
        }

        self.mod_names.insert(module.name().to_owned());
    }

    /// Renames instances with conflicting names within a single module.
    fn rename_instances(&self, module: &mut Module) {
        let mut names = HashSet::new();
        for inst in module.instances_mut() {
            if names.contains(inst.name()) {
                let mut i = 1;
                let name = loop {
                    let name = format!("{}_{}", inst.name(), i);
                    if !names.contains(&*name) {
                        break name;
                    }
                    i += 1;
                };
                inst.set_name(name);
            }
            names.insert(inst.name().to_owned());
        }
    }
}

impl PreprocessedNetlist {
    /// Resolves nested signals in a path, giving the least nested reference to the same signal.
    pub(crate) fn simplify_path(&self, mut path: SignalPathBuf) -> SignalPathBuf {
        if path.insts.is_empty() {
            return path;
        }
        let mut modules = Vec::with_capacity(path.insts.len());
        let mut module = self.top;
        for inst in path.insts.iter().copied() {
            let inst = &self.modules[module].instance_map()[inst];
            module = inst.module().local_id().unwrap();
            modules.push(module);
        }

        // modules[i] is the module corresponding to path.insts[i].
        assert_eq!(modules.len(), path.insts.len());

        let mut slice = path.slice;
        for i in modules.len() - 1..=0 {
            let module = &self.modules[modules[i]];
            let info = &module.signals()[slice.signal];
            if !info.is_port() {
                path.insts.truncate(i + 1);
                return SignalPathBuf::new(path.insts, slice);
            } else {
                let parent = if i == 0 {
                    &self.modules[self.top]
                } else {
                    &self.modules[modules[i - 1]]
                };
                let i = &parent.instance_map()[path.insts[i]];
                let sig = &i.connections()[info.name()];
                slice = sig.index(slice.idx).into_single();
            }
        }
        SignalPathBuf::new(Vec::new(), slice)
    }
}

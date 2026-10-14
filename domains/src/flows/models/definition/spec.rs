// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use super::{Context, Step, Triggers, identifier};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    #[serde(default)]
    pub contexts: BTreeMap<String, Context>,
    #[serde(default)]
    pub triggers: Triggers,
    pub steps: Vec<Step>,
}

impl Spec {
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("spec.steps is empty".to_string());
        }

        for (name, context) in &self.contexts {
            identifier("context", name)?;
            context
                .validate()
                .map_err(|e| format!("context '{}': {}", name, e))?;
        }

        self.triggers.validate()?;

        let mut step_names = HashSet::new();
        let mut used_contexts = HashSet::new();
        for step in &self.steps {
            step.validate()?;

            if !step_names.insert(step.name.as_str()) {
                return Err(format!("step name '{}' is used more than once", step.name));
            }

            if !self.contexts.contains_key(&step.context) {
                return Err(format!(
                    "step '{}' uses unknown context '{}'",
                    step.name, step.context
                ));
            }
            used_contexts.insert(step.context.as_str());
        }

        if let Some(name) = self
            .contexts
            .keys()
            .find(|name| !used_contexts.contains(name.as_str()))
        {
            return Err(format!("context '{}' is not used by any step", name));
        }

        Ok(())
    }
}

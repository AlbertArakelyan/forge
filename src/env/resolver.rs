use std::collections::{HashMap, HashSet};

use crate::env::interpolator::parse_vars;
use crate::state::app_state::AppState;
use crate::state::environment::VarType;

pub enum VarStatus {
    Resolved(String),
    Unresolved,
    Secret,
}

pub struct VarSpan {
    pub start: usize,
    pub end: usize,
    pub variable_name: String,
    pub status: VarStatus,
}

pub struct ResolvedString {
    pub value: String,
    pub spans: Vec<VarSpan>,
}

pub struct EnvResolver {
    pub layers: Vec<HashMap<String, String>>,
    pub secret_keys: HashSet<String>,
}

impl EnvResolver {
    pub fn new(layers: Vec<HashMap<String, String>>, secret_keys: HashSet<String>) -> Self {
        Self { layers, secret_keys }
    }

    /// Resolve a string for display. Secrets are replaced with `••••••••`.
    pub fn resolve(&self, input: &str) -> ResolvedString {
        let var_spans = parse_vars(input);
        if var_spans.is_empty() {
            return ResolvedString {
                value: input.to_string(),
                spans: Vec::new(),
            };
        }

        let mut output = String::with_capacity(input.len());
        let mut spans = Vec::with_capacity(var_spans.len());
        let mut last = 0;

        for (start, end, name) in &var_spans {
            // Push plain text before this variable
            output.push_str(&input[last..*start]);

            let val_out_start = output.len();

            let resolved = self.lookup(name);
            let (replacement, status) = if let Some(val) = resolved {
                if self.secret_keys.contains(name.as_str()) {
                    ("••••••••".to_string(), VarStatus::Secret)
                } else {
                    (val.clone(), VarStatus::Resolved(val))
                }
            } else {
                // Keep the original `{{name}}` text for unresolved
                (input[*start..*end].to_string(), VarStatus::Unresolved)
            };

            output.push_str(&replacement);
            let val_out_end = output.len();

            spans.push(VarSpan {
                start: val_out_start,
                end: val_out_end,
                variable_name: name.clone(),
                status,
            });

            last = *end;
        }

        // Remaining text after last variable
        output.push_str(&input[last..]);

        ResolvedString { value: output, spans }
    }

    /// Resolve a string for HTTP send. Secrets use their real value.
    pub fn resolve_for_send(&self, input: &str) -> String {
        let var_spans = parse_vars(input);
        if var_spans.is_empty() {
            return input.to_string();
        }

        let mut output = String::with_capacity(input.len());
        let mut last = 0;

        for (start, end, name) in &var_spans {
            output.push_str(&input[last..*start]);
            if let Some(val) = self.lookup_secret(name) {
                output.push_str(&val);
            } else {
                // Keep original placeholder for truly unresolved vars
                output.push_str(&input[*start..*end]);
            }
            last = *end;
        }

        output.push_str(&input[last..]);
        output
    }

    /// Look up a variable name across all layers (display version — no secrets).
    fn lookup(&self, name: &str) -> Option<String> {
        for layer in &self.layers {
            if let Some(val) = layer.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    /// Look up a variable including secret values (for sending).
    fn lookup_secret(&self, name: &str) -> Option<String> {
        self.lookup(name)
    }
}

/// Build an `EnvResolver` from the current `AppState`.
/// Priority: active environment variables > OS environment variables.
pub fn resolver_from_state(state: &AppState) -> EnvResolver {
    let mut layers: Vec<HashMap<String, String>> = Vec::new();
    let mut secret_keys: HashSet<String> = HashSet::new();

    // Layer 0: active environment
    if let Some(idx) = state.active_env_idx {
        if let Some(env) = state.environments.get(idx) {
            let mut map = HashMap::new();
            for var in &env.variables {
                if var.enabled {
                    map.insert(var.key.clone(), var.value.clone());
                    if var.var_type == VarType::Secret {
                        secret_keys.insert(var.key.clone());
                    }
                }
            }
            layers.push(map);
        }
    }

    // Layer 1: OS environment variables (lowest priority)
    let os_map: HashMap<String, String> = std::env::vars().collect();
    layers.push(os_map);

    EnvResolver::new(layers, secret_keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_resolver(vars: &[(&str, &str)], secrets: &[&str]) -> EnvResolver {
        let mut map = HashMap::new();
        for (k, v) in vars {
            map.insert(k.to_string(), v.to_string());
        }
        let secret_keys = secrets.iter().map(|s| s.to_string()).collect();
        EnvResolver::new(vec![map], secret_keys)
    }

    #[test]
    fn test_resolve_found() {
        let r = make_resolver(&[("host", "example.com")], &[]);
        let result = r.resolve("{{host}}/api");
        assert_eq!(result.value, "example.com/api");
        assert_eq!(result.spans.len(), 1);
        assert!(matches!(result.spans[0].status, VarStatus::Resolved(_)));
    }

    #[test]
    fn test_resolve_not_found() {
        let r = make_resolver(&[], &[]);
        let result = r.resolve("{{unknown}}/api");
        // Unresolved keeps the placeholder text
        assert!(result.value.contains("{{unknown}}"));
        assert_eq!(result.spans.len(), 1);
        assert!(matches!(result.spans[0].status, VarStatus::Unresolved));
    }

    #[test]
    fn test_resolve_secret_display() {
        let r = make_resolver(&[("token", "supersecret")], &["token"]);
        let result = r.resolve("Bearer {{token}}");
        // Display shows bullets
        assert_eq!(result.value, "Bearer ••••••••");
        assert!(matches!(result.spans[0].status, VarStatus::Secret));
    }

    #[test]
    fn test_resolve_for_send_secret_real_value() {
        let r = make_resolver(&[("token", "supersecret")], &["token"]);
        let result = r.resolve_for_send("Bearer {{token}}");
        assert_eq!(result, "Bearer supersecret");
    }

    #[test]
    fn test_resolve_for_send_found() {
        let r = make_resolver(&[("host", "example.com")], &[]);
        let result = r.resolve_for_send("{{host}}/api");
        assert_eq!(result, "example.com/api");
    }
}

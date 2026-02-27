use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VarType {
    #[default]
    Text,
    Secret,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    pub key: String,
    pub value: String,
    pub var_type: VarType,
    pub enabled: bool,
    pub description: String,
}

impl Default for EnvVariable {
    fn default() -> Self {
        Self {
            key: String::new(),
            value: String::new(),
            var_type: VarType::Text,
            enabled: true,
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub color: String,
    pub variables: Vec<EnvVariable>,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::from("New Environment"),
            color: String::from("#7aa2f7"),
            variables: Vec::new(),
        }
    }
}

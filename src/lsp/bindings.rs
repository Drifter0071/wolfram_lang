use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct WoldFile {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub globals: Vec<WoldGlobal>,
    #[serde(default)]
    pub functions: Vec<WoldFunction>,
    #[serde(default)]
    pub types: Vec<WoldType>,
    #[serde(default)]
    pub enums: Vec<WoldEnum>,
    #[serde(default)]
    pub services: Vec<WoldService>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldGlobal {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldFunction {
    pub name: String,
    #[serde(default)]
    pub params: Vec<WoldParam>,
    pub returns: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldParam {
    pub name: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldProperty {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub rw: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldMethod {
    pub name: String,
    #[serde(default)]
    pub params: Vec<WoldParam>,
    #[serde(default)]
    pub returns: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldEvent {
    pub name: String,
    #[serde(default)]
    pub params: Vec<WoldParam>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldType {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub properties: Vec<WoldProperty>,
    #[serde(default)]
    pub methods: Vec<WoldMethod>,
    #[serde(default)]
    pub events: Vec<WoldEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldEnum {
    pub name: String,
    pub items: Vec<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct WoldService {
    pub name: String,
    pub class_name: String,
    #[serde(default)]
    pub description: String,
}

pub struct Bindings {
    pub globals: Vec<WoldGlobal>,
    pub functions: Vec<WoldFunction>,
    types: Vec<WoldType>,
    pub enums: Vec<WoldEnum>,
    pub services: Vec<WoldService>,
    type_index: HashMap<String, usize>,
}

impl Bindings {
    pub fn empty() -> Self {
        Self {
            globals: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            enums: Vec::new(),
            services: Vec::new(),
            type_index: HashMap::new(),
        }
    }

    pub fn load(bindings_path: Option<&str>) -> Self {
        let mut bindings = Self::empty();
        if let Some(dir) = bindings_path {
            let p = Path::new(dir).join("generated").join("roblox.wold");
            if let Ok(data) = fs::read_to_string(&p) {
                if let Ok(wold) = serde_json::from_str::<WoldFile>(&data) {
                    bindings.merge(wold);
                    eprintln!("Loaded bindings from {}", p.display());
                }
            }
        } else {
            eprintln!("LSP: no --bindings path provided, skipping built-in bindings");
        }
        bindings
    }

    pub fn load_workspace_wolds(&mut self, root: &str) {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("wold") {
                    if let Ok(data) = fs::read_to_string(&path) {
                        if let Ok(wold) = serde_json::from_str::<WoldFile>(&data) {
                            self.merge(wold);
                        }
                    }
                }
            }
        }
    }

    fn merge(&mut self, file: WoldFile) {
        self.globals.extend(file.globals);
        self.functions.extend(file.functions);
        self.enums.extend(file.enums);
        self.services.extend(file.services);
        for t in file.types {
            let name_lower = t.name.to_lowercase();
            self.type_index.insert(name_lower, self.types.len());
            self.types.push(t);
        }
    }

    pub fn get_type(&self, name: &str) -> Option<&WoldType> {
        let lower = name.to_lowercase();
        self.type_index.get(&lower).map(|&i| &self.types[i])
    }

    pub fn get_global(&self, name: &str) -> Option<&WoldGlobal> {
        self.globals.iter().find(|g| g.name.eq_ignore_ascii_case(name))
    }

    pub fn get_all_methods(&self, type_name: &str) -> Vec<&WoldMethod> {
        let mut methods = Vec::new();
        if let Some(t) = self.get_type(type_name) {
            methods.extend(t.methods.iter());
            if let Some(parent) = &t.extends {
                methods.extend(self.get_all_methods(parent));
            }
        }
        methods
    }

    pub fn get_all_properties(&self, type_name: &str) -> Vec<&WoldProperty> {
        let mut props = Vec::new();
        if let Some(t) = self.get_type(type_name) {
            props.extend(t.properties.iter());
            if let Some(parent) = &t.extends {
                props.extend(self.get_all_properties(parent));
            }
        }
        props
    }

    pub fn get_method_return(&self, type_name: &str, method_name: &str) -> Option<String> {
        if let Some(t) = self.get_type(type_name) {
            if let Some(m) = t.methods.iter().find(|m| m.name.eq_ignore_ascii_case(method_name)) {
                return Some(m.returns.clone());
            }
            if let Some(parent) = &t.extends {
                return self.get_method_return(parent, method_name);
            }
        }
        None
    }
}

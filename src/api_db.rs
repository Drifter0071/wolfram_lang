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
    #[serde(default)]
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
pub struct WoldType {
    pub name: String,
    #[serde(default)]
    pub description: String,
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
pub struct WoldEvent {
    pub name: String,
    #[serde(default)]
    pub params: Vec<WoldParam>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldEnum {
    pub name: String,
    pub items: Vec<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoldService {
    pub name: String,
    pub class_name: String,
    #[serde(default)]
    pub description: String,
}

pub struct ApiDatabase {
    pub globals: HashMap<String, WoldGlobal>,
    pub functions: HashMap<String, WoldFunction>,
    types: Vec<WoldType>,
    type_index: HashMap<String, usize>,
    pub enums: HashMap<String, WoldEnum>,
    pub services: HashMap<String, WoldService>,
    loaded: bool,
}

impl ApiDatabase {
    pub fn empty() -> Self {
        Self {
            globals: HashMap::new(),
            functions: HashMap::new(),
            types: Vec::new(),
            type_index: HashMap::new(),
            enums: HashMap::new(),
            services: HashMap::new(),
            loaded: false,
        }
    }

    pub fn load_from_file(path: &Path) -> Self {
        let mut db = Self::empty();
        match fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str::<WoldFile>(&raw) {
                Ok(wold) => {
                    for g in wold.globals {
                        db.globals.insert(g.name.to_lowercase(), g);
                    }
                    for f in wold.functions {
                        db.functions.insert(f.name.to_lowercase(), f);
                    }
                    for t in wold.types {
                        let key = t.name.to_lowercase();
                        db.type_index.insert(key, db.types.len());
                        db.types.push(t);
                    }
                    for e in wold.enums {
                        db.enums.insert(e.name.to_lowercase(), e);
                    }
                    for s in wold.services {
                        db.services.insert(s.name.to_lowercase(), s);
                    }
                    db.loaded = true;
                }
                Err(e) => {
                    eprintln!("[api_db] Failed to parse wold: {}", e);
                }
            },
            Err(e) => {
                eprintln!("[api_db] Failed to read wold file: {}", e);
            }
        }
        db
    }

    pub fn load_from_directory(dir: &Path) -> Self {
        let wold_path = dir.join("generated").join("roblox.wold");
        Self::load_from_file(&wold_path)
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn get_type(&self, name: &str) -> Option<&WoldType> {
        self.type_index
            .get(&name.to_lowercase())
            .and_then(|&i| self.types.get(i))
    }

    pub fn get_global(&self, name: &str) -> Option<&WoldGlobal> {
        self.globals.get(&name.to_lowercase())
    }

    pub fn get_global_type(&self, name: &str) -> Option<String> {
        self.get_global(name).map(|g| g.r#type.clone())
    }

    pub fn is_known_class(&self, name: &str) -> bool {
        self.type_index.contains_key(&name.to_lowercase())
    }

    pub fn property_info(&self, class_name: &str, prop_name: &str) -> Option<&WoldProperty> {
        self.get_type(class_name)?
            .properties
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(prop_name))
    }

    pub fn property_type(&self, class_name: &str, prop_name: &str) -> Option<String> {
        self.property_info(class_name, prop_name)
            .map(|p| p.r#type.clone())
    }

    pub fn method_info(&self, class_name: &str, method: &str) -> Option<&WoldMethod> {
        let mut current = class_name.to_string();
        loop {
            if let Some(t) = self.get_type(&current) {
                for m in &t.methods {
                    if m.name.eq_ignore_ascii_case(method) {
                        return Some(m);
                    }
                }
                match &t.extends {
                    Some(parent) => current = parent.clone(),
                    None => break,
                }
            } else {
                break;
            }
        }
        None
    }

    pub fn method_returns(&self, class_name: &str, method: &str) -> Option<String> {
        self.method_info(class_name, method)
            .map(|m| m.returns.clone())
    }

    pub fn property_exists(&self, class_name: &str, prop_name: &str) -> bool {
        let mut current = class_name.to_string();
        loop {
            if let Some(t) = self.get_type(&current) {
                if t.properties
                    .iter()
                    .any(|p| p.name.eq_ignore_ascii_case(prop_name))
                {
                    return true;
                }
                match &t.extends {
                    Some(parent) => current = parent.clone(),
                    None => break,
                }
            } else {
                break;
            }
        }
        false
    }

    pub fn method_exists(&self, class_name: &str, method: &str) -> bool {
        let mut current = class_name.to_string();
        loop {
            if let Some(t) = self.get_type(&current) {
                if t.methods
                    .iter()
                    .any(|m| m.name.eq_ignore_ascii_case(method))
                {
                    return true;
                }
                match &t.extends {
                    Some(parent) => current = parent.clone(),
                    None => break,
                }
            } else {
                break;
            }
        }
        false
    }

    pub fn get_function(&self, name: &str) -> Option<&WoldFunction> {
        self.functions.get(&name.to_lowercase())
    }

    pub fn is_deprecated(&self, _name: &str) -> bool {
        false
    }

    pub fn get_deprecation(&self, _name: &str) -> Option<String> {
        None
    }
}

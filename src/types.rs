use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum InferredType {
    Number,
    String,
    Bool,
    Nil,
    Unknown,
    Array(Box<InferredType>),
    Table,
    Function,
    Instance,
    Vector3,
    Vector2,
    CFrame,
    UDim2,
    UDim,
    Color3,
    BrickColor,
}

impl InferredType {
    pub fn name(&self) -> &'static str {
        match self {
            InferredType::Number => "number",
            InferredType::String => "string",
            InferredType::Bool => "boolean",
            InferredType::Nil => "nil",
            InferredType::Unknown => "unknown",
            InferredType::Array(_) => "array",
            InferredType::Table => "table",
            InferredType::Function => "function",
            InferredType::Instance => "Instance",
            InferredType::Vector3 => "Vector3",
            InferredType::Vector2 => "Vector2",
            InferredType::CFrame => "CFrame",
            InferredType::UDim2 => "UDim2",
            InferredType::UDim => "UDim",
            InferredType::Color3 => "Color3",
            InferredType::BrickColor => "BrickColor",
        }
    }
}

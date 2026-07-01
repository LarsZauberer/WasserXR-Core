use std::{collections::HashMap, fs};

use glium::uniforms::AsUniformValue;
use serde_json::Value;
use wasserxr::{asset_type, asset_type_creator, scene::Scene, utils::paths::get_asset_path, warn};

pub enum MaterialData {
    Float(f32),
    Int(i32),
    Vec4([f32; 4]),
    Vec3([f32; 3]),
    Vec2([f32; 2]),
}

impl AsUniformValue for MaterialData {
    fn as_uniform_value(&self) -> glium::uniforms::UniformValue<'_> {
        match self {
            MaterialData::Float(v) => glium::uniforms::UniformValue::Float(*v),
            MaterialData::Int(v) => glium::uniforms::UniformValue::SignedInt(*v),
            MaterialData::Vec4(v) => glium::uniforms::UniformValue::Vec4(*v),
            MaterialData::Vec3(v) => glium::uniforms::UniformValue::Vec3(*v),
            MaterialData::Vec2(v) => glium::uniforms::UniformValue::Vec2(*v),
        }
    }
}

#[asset_type]
struct MaterialAsset {
    shader: String,
    data: HashMap<String, MaterialData>,
}

impl MaterialAsset {
    pub fn new(shader: String) -> Self {
        Self {
            shader,
            data: HashMap::default(),
        }
    }

    pub fn add_data(&mut self, key: String, data: MaterialData) {
        self.data.insert(key, data);
    }
}

#[asset_type_creator(MaterialAsset)]
fn create_material(scene: &mut Scene, data: &str) -> Option<MaterialAsset> {
    let Some(file_path) = get_asset_path(data) else {
        warn!(scene, "Failed to find material: {}", data);
        return None;
    };

    let Ok(raw_material_data) = fs::read_to_string(file_path) else {
        warn!(scene, "Failed to read material: {}", data);
        return None;
    };

    let Ok(material_data) = serde_json::from_str::<Value>(&raw_material_data) else {
        warn!(scene, "Material is not a valid json: {}", data);
        return None;
    };

    let Some(material_data) = material_data.as_object() else {
        warn!(scene, "Material is not a json object: {}", data);
        return None;
    };

    let Some(shader) = material_data.get("shader").and_then(Value::as_str) else {
        warn!(scene, "Material does not define a shader: {}", data);
        return None;
    };

    let mut material = MaterialAsset::new(shader.to_string());

    for (key, value) in material_data {
        if key == "shader" {
            continue;
        }

        let material_data = match value {
            Value::Number(number) => {
                if let Some(value) = number
                    .as_i64()
                    .and_then(|value| i32::try_from(value).ok())
                    .or_else(|| number.as_u64().and_then(|value| i32::try_from(value).ok()))
                {
                    MaterialData::Int(value)
                } else if let Some(value) = number.as_f64() {
                    MaterialData::Float(value as f32)
                } else {
                    warn!(scene, "Invalid material data `{}` in {}", key, data);
                    continue;
                }
            }
            Value::Array(values) if values.len() == 2 => {
                let (Some(x), Some(y)) = (values[0].as_f64(), values[1].as_f64()) else {
                    warn!(scene, "Invalid material data `{}` in {}", key, data);
                    continue;
                };
                MaterialData::Vec2([x as f32, y as f32])
            }
            Value::Array(values) if values.len() == 3 => {
                let (Some(x), Some(y), Some(z)) =
                    (values[0].as_f64(), values[1].as_f64(), values[2].as_f64())
                else {
                    warn!(scene, "Invalid material data `{}` in {}", key, data);
                    continue;
                };
                MaterialData::Vec3([x as f32, y as f32, z as f32])
            }
            Value::Array(values) if values.len() == 4 => {
                let (Some(x), Some(y), Some(z), Some(a)) = (
                    values[0].as_f64(),
                    values[1].as_f64(),
                    values[2].as_f64(),
                    values[3].as_f64(),
                ) else {
                    warn!(scene, "Invalid material data `{}` in {}", key, data);
                    continue;
                };
                MaterialData::Vec4([x as f32, y as f32, z as f32, a as f32])
            }
            _ => {
                warn!(scene, "Invalid material data `{}` in {}", key, data);
                continue;
            }
        };

        material.add_data(key.clone(), material_data);
    }

    Some(material)
}

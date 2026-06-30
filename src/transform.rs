use std::default;

use wasserxr::component;

#[component]
pub struct Transform {
    #[mutable]
    pub position: [f32; 3],

    #[mutable]
    pub rotation: [f32; 3],

    #[mutable]
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

#[cfg(test)]
mod tests {
    use wasserxr::scene::Scene;

    #[test]
    fn transform_component_defaults_and_mutates_fields() {
        let mut scene = Scene::new();

        let entity = scene.add_entity();

        scene.add_component(entity, "Transform".to_owned()).unwrap();

        let (position, rotation, scale) = scene
            .query::<(&[f32; 3], &[f32; 3], &[f32; 3])>(
                entity,
                "Transform",
                &["position", "rotation", "scale"],
            )
            .unwrap();
        assert_eq!(*position, [0.0, 0.0, 0.0]);
        assert_eq!(*rotation, [0.0, 0.0, 0.0]);
        assert_eq!(*scale, [0.0, 0.0, 0.0]);

        {
            let (position, rotation, scale) = scene
                .query_mut::<(&mut [f32; 3], &mut [f32; 3], &mut [f32; 3])>(
                    entity,
                    "Transform",
                    &["position", "rotation", "scale"],
                )
                .unwrap();

            *position = [1.0, 2.0, 3.0];
            *rotation = [4.0, 5.0, 6.0];
            *scale = [7.0, 8.0, 9.0];
        }

        let (position, rotation, scale) = scene
            .query::<(&[f32; 3], &[f32; 3], &[f32; 3])>(
                entity,
                "Transform",
                &["position", "rotation", "scale"],
            )
            .unwrap();
        assert_eq!(*position, [1.0, 2.0, 3.0]);
        assert_eq!(*rotation, [4.0, 5.0, 6.0]);
        assert_eq!(*scale, [7.0, 8.0, 9.0]);

        scene.remove_component(entity, "Transform").unwrap();
    }
}

use wasserxr::component;

#[component]
#[derive(Default)]
pub struct Transform {
    #[mutable]
    pub location: [f32; 3],

    #[mutable]
    pub rotation: [f32; 3],

    #[mutable]
    pub scale: [f32; 3],
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use wasserxr::{scene::Scene, scene::component::Schema};

    #[test]
    fn transform_component_defaults_and_mutates_fields() {
        retain_static_symbols();

        let mut scene = Scene::new();

        let entity = scene.add_entity();

        scene.add_component(entity, "Transform".to_owned()).unwrap();

        let (location, rotation, scale) = scene
            .query::<(&[f32; 3], &[f32; 3], &[f32; 3])>(
                entity,
                "Transform",
                &["location", "rotation", "scale"],
            )
            .unwrap();
        assert_eq!(*location, [0.0, 0.0, 0.0]);
        assert_eq!(*rotation, [0.0, 0.0, 0.0]);
        assert_eq!(*scale, [0.0, 0.0, 0.0]);

        {
            let (location, rotation, scale) = scene
                .query_mut::<(&mut [f32; 3], &mut [f32; 3], &mut [f32; 3])>(
                    entity,
                    "Transform",
                    &["location", "rotation", "scale"],
                )
                .unwrap();

            *location = [1.0, 2.0, 3.0];
            *rotation = [4.0, 5.0, 6.0];
            *scale = [7.0, 8.0, 9.0];
        }

        let (location, rotation, scale) = scene
            .query::<(&[f32; 3], &[f32; 3], &[f32; 3])>(
                entity,
                "Transform",
                &["location", "rotation", "scale"],
            )
            .unwrap();
        assert_eq!(*location, [1.0, 2.0, 3.0]);
        assert_eq!(*rotation, [4.0, 5.0, 6.0]);
        assert_eq!(*scale, [7.0, 8.0, 9.0]);

        scene.remove_component(entity, "Transform").unwrap();
    }

    fn retain_static_symbols() {
        std::hint::black_box((
            super::wxr_create_Transform as unsafe extern "C" fn() -> *mut c_void,
            super::wxr_destroy_Transform as unsafe extern "C" fn(*mut c_void),
            super::wxr_schema_Transform as unsafe extern "C" fn(*mut Schema),
            super::wxr_get_Transform_location as unsafe extern "C" fn(*mut c_void) -> *mut c_void,
            super::wxr_get_Transform_rotation as unsafe extern "C" fn(*mut c_void) -> *mut c_void,
            super::wxr_get_Transform_scale as unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        ));
    }
}

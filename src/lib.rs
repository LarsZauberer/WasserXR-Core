use wasserxr::{Uuid, scene::Scene, system};

#[system]
pub fn hello_world(_scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    println!("Hello WasserXR!");
}

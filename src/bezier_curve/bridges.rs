use std::collections::HashMap;

use crate::{
    picking3d::{events::Pointer3d, picking_3d::Picking3dInteractable},
    projection::{AddBoundingEntityEvent, DisplayIn},
    translation_control::{
        enable_gizmo, enable_gizmo3d,
        translation_controller::{
            CantSnapToCurve, CantSnapToEntities, EnableTranslationControl,
            EnableTranslationControlType, MovedEntityEvent, SnappedPoint,
        },
    },
    util::update_material_on,
};

use super::{
    bezier_curve_renderer::{generic_on_despawn_trigger, hover_3d},
    render_info::RenderInformation,
};
use bevy::{
    color::palettes::{
        css::BLACK,
        tailwind::{GREEN_600, GREEN_800},
    },
    ecs::system::{SystemParam, lifetimeless::Read},
    prelude::*,
    render::view::RenderLayers,
};

#[derive(Component)]
pub struct Bridge {
    pub a: Entity,
    pub b: Entity,
}

impl Default for Bridge {
    fn default() -> Self {
        Self {
            a: Entity::PLACEHOLDER,
            b: Entity::PLACEHOLDER,
        }
    }
}

#[derive(Component)]
#[relationship(relationship_target = CompleteBridgeCenters)]
#[component(on_despawn = generic_on_despawn_trigger)]
pub struct CompleteBridgeCenter {
    #[relationship]
    pub complete_bridge: Entity,
    pub single_segment: Option<Entity>,
}

#[derive(Component)]
#[relationship_target(relationship = CompleteBridgeCenter, linked_spawn)]
pub struct CompleteBridgeCenters(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = CompleteBridge)]
#[require(Bridge)]
pub struct BridgeConnector(pub Entity);

#[derive(Component, Default)]
#[relationship_target(relationship=BridgeConnector, linked_spawn)]
pub struct CompleteBridge(Vec<Entity>);

impl CompleteBridge {
    pub fn get_bridges(&self) -> &'_ Vec<Entity> {
        &self.0
    }
}

#[derive(Component)]
pub struct BridgeMarker;

#[allow(clippy::complexity)]
pub fn update_lines(
    mut commands: Commands,
    mut lines: Query<
        (&Bridge, Entity, &Children, &mut Transform),
        (Without<BridgeMarker>, Without<CompleteBridgeCenter>),
    >,
    mut changeable: Query<
        (&mut Transform, &mut Mesh3d),
        (
            Without<Bridge>,
            With<BridgeMarker>,
            Without<CompleteBridgeCenter>,
        ),
    >,
    mut bridge_centers: Query<
        (&mut Transform, &CompleteBridgeCenter),
        (Without<BridgeMarker>, Without<Bridge>),
    >,
    transforms: Query<
        &Transform,
        (
            Without<BridgeMarker>,
            Without<Bridge>,
            Without<CompleteBridgeCenter>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    info: Res<RenderInformation>,
) {
    for (line, line_entity, children, mut transform) in lines.iter_mut() {
        let (entity1, entity2) = {
            let entity1 = transforms.get(line.a);
            let entity2 = transforms.get(line.b);
            (entity1, entity2)
        };

        if entity1.is_err() || entity2.is_err() {
            let _ = commands.get_entity(line_entity).map(|mut e| e.despawn());
            continue;
        }

        let origin = entity1.unwrap().translation;
        let target = entity2.unwrap().translation;
        let diff = target - origin;
        let length = diff.length();

        for child in children {
            if let Ok((mut transform, mut mesh3d)) = changeable.get_mut(*child) {
                let cylinder = meshes.add(Cylinder::new(0.02 * info.scale, length));
                *transform = Transform::from_xyz(0.0, 0.0, -length / 2.0)
                    .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians()));
                mesh3d.0 = cylinder;
            }
        }

        *transform = Transform::from_translation(origin).looking_at(target, Vec3::Y);
    }

    for (mut transform, center) in bridge_centers.iter_mut() {
        if let Some(segment) = center.single_segment
            && let Ok(line) = lines.get(segment)
        {
            let a = line.0.a;
            let b = line.0.b;

            let transform_a = transforms.get(a).unwrap().translation;
            let transform_b = transforms.get(b).unwrap().translation;

            let midpoint = transform_a * 0.5 + transform_b * 0.5;

            transform.translation = midpoint;
        }
    }
}

fn moved_complete_bridge(
    trigger: Trigger<MovedEntityEvent>,
    centers: Query<&CompleteBridgeCenter>,
    complete_brigdes: Query<&CompleteBridge>,
    bridge_segments: Query<&Bridge>,
    mut commands: Commands,
    mut transforms: Query<&mut Transform>,
) {
    let Ok(center) = centers.get(trigger.target()) else {
        return;
    };

    let Ok(bridges) = complete_brigdes.get(center.complete_bridge) else {
        return;
    };

    let mut entity_transform_mapping = HashMap::new();
    for bridge in bridges.get_bridges() {
        if let Ok(bridge) = bridge_segments.get(*bridge) {
            if let Ok(transform) = transforms.get(bridge.a) {
                entity_transform_mapping.insert(bridge.a, *transform);
            }
            if let Ok(transform) = transforms.get(bridge.b) {
                entity_transform_mapping.insert(bridge.b, *transform);
            }
        }
    }

    for v in entity_transform_mapping.values_mut() {
        v.translation += trigger.event().delta;
    }

    // apply edited translations to world
    for (k, v) in &entity_transform_mapping {
        if let Ok(mut transform) = transforms.get_mut(*k) {
            transform.translation = v.translation;
        }

        // This might drag points away from curves, meaning snapping must end here
        let _ = commands.get_entity(*k).map(|mut e| {
            e.remove::<SnappedPoint>();
        });
    }

    // compute the new position of the sphere that defines the bridges movement
    if let Ok(mut transform) = transforms.get_mut(trigger.target())
        && let Ok(segment) = bridge_segments.get(center.single_segment.expect("Must be present"))
    {
        let transform_a = entity_transform_mapping[&segment.a].translation;
        let transform_b = entity_transform_mapping[&segment.b].translation;

        let diff = transform_b - transform_a;

        transform.translation = transform_a + 0.5 * diff;
    }
}

pub fn draw_bridge_cylinder(
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    length: f32,
    scale: f32,
) -> impl Bundle {
    let cylinder = meshes.add(Cylinder::new(0.02 * scale, length));
    let material = materials.add(StandardMaterial::from_color(BLACK));

    (
        Mesh3d(cylinder),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, -length / 2.0)
            .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())),
        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
        Visibility::Inherited,
        BridgeMarker,
    )
}

#[derive(SystemParam)]
pub struct BridgeSpawner<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    render_info: Res<'w, RenderInformation>,
    add_bounding_entities: EventWriter<'w, AddBoundingEntityEvent>,
}

impl<'w, 's> BridgeSpawner<'w, 's> {
    pub fn spawn_bridges_2d(
        &mut self,
        parent: Entity,
        w: usize,
        h: usize,
        points: &[(usize, usize, Vec3)],
        ids: &[Entity],
    ) {
        let scale = self.render_info.scale;

        let mut x_bridges = Vec::new();
        let mut y_bridges = Vec::new();

        for x in 0..w {
            x_bridges.push(
                self.commands
                    .spawn((
                        CompleteBridge::default(),
                        Name::new(format!("x_bridge {x}")),
                        Transform::default(),
                        Visibility::Inherited,
                        ChildOf(parent),
                    ))
                    .id(),
            );
        }

        for y in 0..h {
            y_bridges.push(
                self.commands
                    .spawn((
                        CompleteBridge::default(),
                        Name::new(format!("y_bridge {y}")),
                        Transform::default(),
                        Visibility::Inherited,
                        ChildOf(parent),
                    ))
                    .id(),
            );
        }

        let to_xy = |i| (i % w, i / w);
        let from_xy = |x, y| x + y * w;

        let x_mid = w / 2;
        let y_mid = h / 2;
        // Draw lines between neighbouring controls points to generate a visible grid
        for i in 0..points.len() {
            let p0 = points[i];
            let p0_id = ids[i];

            let (x, y) = to_xy(i);
            let (_, y_next) = to_xy(i + 1);

            // Check if we are not at the edge of the array (next index is one row up)
            if y == y_next
                && let Some(px) = points.get(i + 1)
            {
                let id = *ids.get(i + 1).expect("must be present");

                let origin = p0.2;
                let target = px.2;
                let diff = target - origin;

                let bridge_id = self
                    .commands
                    .spawn((
                        Transform::from_translation(origin).looking_at(target, Vec3::Y),
                        Bridge { a: p0_id, b: id },
                        BridgeConnector(y_bridges[y]),
                        Name::new(format!("Render line {p0_id} {id}")),
                        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                        Visibility::Inherited,
                        ChildOf(parent),
                        related!(
                            Children[draw_bridge_cylinder(
                                &mut self.meshes,
                                &mut self.materials,
                                diff.length(),
                                scale
                            )]
                        ),
                    ))
                    .id();

                let is_x_mid = x < x_mid && x + 1 >= x_mid;
                if is_x_mid {
                    let complete_bridge = y_bridges[y];

                    let center = origin + diff * 0.5;
                    let material = self.materials.add(Color::from(GREEN_800));
                    let material_hover = self.materials.add(Color::from(GREEN_600));

                    let id = self
                        .commands
                        .spawn((
                            Transform::from_translation(center),
                            CompleteBridgeCenter {
                                complete_bridge,
                                single_segment: Some(bridge_id),
                            },
                            ChildOf(parent),
                            Visibility::Inherited,
                            Mesh3d(self.meshes.add(Sphere::new(0.1 * scale))),
                            MeshMaterial3d(material.clone()),
                            Picking3dInteractable::default(),
                            CantSnapToEntities::All,
                        ))
                        .observe(enable_gizmo(EnableTranslationControl::new_with_root(
                            EnableTranslationControlType::OnlyTranslation,
                        )))
                        .observe(enable_gizmo3d(EnableTranslationControl::new_with_root(
                            EnableTranslationControlType::OnlyTranslation,
                        )))
                        .observe(update_material_on::<
                            Pointer3d<crate::picking3d::events::MoveIn>,
                        >(material_hover.clone()))
                        .observe(update_material_on::<
                            Pointer3d<crate::picking3d::events::MoveIn>,
                        >(material.clone()))
                        .observe(hover_3d)
                        .observe(moved_complete_bridge)
                        .id();
                    self.add_bounding_entities.write(AddBoundingEntityEvent(id));
                }
            }

            let next_row_idx = from_xy(x, y + 1);
            // Check if next row is still in range
            if y + 1 < h
                && let Some(py) = points.get(next_row_idx)
            {
                let id = *ids.get(i + w).expect("must be present");

                let origin = p0.2;
                let target = py.2;
                let diff = target - origin;

                let bridge_id = self
                    .commands
                    .spawn((
                        Transform::from_translation(origin).looking_at(target, Vec3::Y),
                        Bridge { a: p0_id, b: id },
                        BridgeConnector(x_bridges[x]),
                        Name::new(format!("Render line {p0_id} {id}")),
                        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                        Visibility::Inherited,
                        ChildOf(parent),
                        related!(
                            Children[draw_bridge_cylinder(
                                &mut self.meshes,
                                &mut self.materials,
                                diff.length(),
                                scale
                            )]
                        ),
                    ))
                    .id();

                let is_y_mid = y < y_mid && y + 1 >= y_mid;
                if is_y_mid {
                    let center = origin + diff * 0.5;
                    let complete_bridge = x_bridges[x];

                    let material = self.materials.add(Color::from(GREEN_800));
                    let material_hover = self.materials.add(Color::from(GREEN_600));

                    let id = self
                        .commands
                        .spawn((
                            Transform::from_translation(center),
                            CompleteBridgeCenter {
                                complete_bridge,
                                single_segment: Some(bridge_id),
                            },
                            ChildOf(parent),
                            Visibility::Inherited,
                            Mesh3d(self.meshes.add(Sphere::new(0.1 * scale))),
                            MeshMaterial3d(material.clone()),
                            Picking3dInteractable::default(),
                            CantSnapToEntities::All,
                            CantSnapToCurve::Single(parent),
                        ))
                        .observe(enable_gizmo(EnableTranslationControl::new_with_root(
                            EnableTranslationControlType::OnlyTranslation,
                        )))
                        .observe(enable_gizmo3d(EnableTranslationControl::new_with_root(
                            EnableTranslationControlType::OnlyTranslation,
                        )))
                        .observe(update_material_on::<
                            Pointer3d<crate::picking3d::events::MoveIn>,
                        >(material_hover.clone()))
                        .observe(update_material_on::<
                            Pointer3d<crate::picking3d::events::MoveIn>,
                        >(material.clone()))
                        .observe(hover_3d)
                        .observe(moved_complete_bridge)
                        .id();
                    self.add_bounding_entities.write(AddBoundingEntityEvent(id));
                }
            }

            // let is_y_mid = y > y_mid && y < y_mid + 1;
        }
    }

    pub fn spawn_bridges_1d(
        &mut self,
        parent: Entity,
        l: usize,
        points: &[(usize, Vec3)],
        ids: &[Entity],
    ) {
        let scale = self.render_info.scale;
        let mid = l / 2;

        let bridge = self
            .commands
            .spawn((
                CompleteBridge::default(),
                Name::new(format!("bridge for {parent}")),
                Transform::default(),
                Visibility::Inherited,
                ChildOf(parent),
            ))
            .id();

        for i in 0..points.len() - 1 {
            let p = points[i];
            let id = ids[i];

            let next_p = points[i + 1];
            let next_id = ids[i + 1];

            let origin = p.1;
            let target = next_p.1;
            let diff = target - origin;

            let bridge_id = self
                .commands
                .spawn((
                    Transform::from_translation(origin).looking_at(target, Vec3::Y),
                    Bridge { a: id, b: next_id },
                    BridgeConnector(bridge),
                    Name::new(format!("Render line {id} {next_id}")),
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                    Visibility::Inherited,
                    ChildOf(parent),
                    related!(
                        Children[draw_bridge_cylinder(
                            &mut self.meshes,
                            &mut self.materials,
                            diff.length(),
                            scale
                        )]
                    ),
                ))
                .id();

            let is_l_mid = i < mid && i + 1 >= mid;
            if is_l_mid {
                let complete_bridge = bridge;

                let center = origin + diff * 0.5;
                let material = self.materials.add(Color::from(GREEN_800));
                let material_hover = self.materials.add(Color::from(GREEN_600));

                let id = self
                    .commands
                    .spawn((
                        Transform::from_translation(center),
                        CompleteBridgeCenter {
                            complete_bridge,
                            single_segment: Some(bridge_id),
                        },
                        ChildOf(parent),
                        Visibility::Inherited,
                        Mesh3d(self.meshes.add(Sphere::new(0.1 * scale))),
                        MeshMaterial3d(material.clone()),
                        Picking3dInteractable::default(),
                        CantSnapToCurve::Single(parent),
                        CantSnapToEntities::All,
                    ))
                    .observe(enable_gizmo(EnableTranslationControl::new_with_root(
                        EnableTranslationControlType::OnlyTranslation,
                    )))
                    .observe(enable_gizmo3d(EnableTranslationControl::new_with_root(
                        EnableTranslationControlType::OnlyTranslation,
                    )))
                    .observe(update_material_on::<
                        Pointer3d<crate::picking3d::events::MoveIn>,
                    >(material_hover.clone()))
                    .observe(update_material_on::<
                        Pointer3d<crate::picking3d::events::MoveIn>,
                    >(material.clone()))
                    .observe(hover_3d)
                    .observe(moved_complete_bridge)
                    .id();
                self.add_bounding_entities.write(AddBoundingEntityEvent(id));
            }
        }
    }
}

#[derive(SystemParam)]
pub struct BridgeDespawner<'w, 's> {
    commands: Commands<'w, 's>,
    children: Query<'w, 's, Read<Children>>,
    bridges: Query<'w, 's, Entity, With<Bridge>>,
    bridge_centers: Query<'w, 's, Entity, With<CompleteBridgeCenter>>,
    complete_bridges: Query<'w, 's, Entity, With<CompleteBridge>>,
}

impl<'w, 's> BridgeDespawner<'w, 's> {
    pub fn despawn_from_parent(&mut self, parent: Entity) {
        if let Ok(children) = self.children.get(parent) {
            // First delete bridges
            for child in children {
                if let Ok(entity) = self.bridges.get(*child) {
                    let _ = self.commands.get_entity(entity).map(|mut e| e.despawn());
                }
            }

            // Then delete Centers
            for child in children {
                if let Ok(entity) = self.bridge_centers.get(*child) {
                    let _ = self.commands.get_entity(entity).map(|mut e| e.despawn());
                }
            }

            // Last, delete Complete Bridges
            for child in children {
                if let Ok(entity) = self.complete_bridges.get(*child) {
                    let _ = self.commands.get_entity(entity).map(|mut e| e.despawn());
                }
            }
        }
    }
}

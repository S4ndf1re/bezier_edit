use bevy::{
    color::palettes::tailwind::{YELLOW_400, YELLOW_600},
    ecs::system::{
        SystemParam,
        lifetimeless::{Read, Write},
    },
    prelude::*,
};

use crate::{
    bezier_curve::{
        bezier_curve_renderer::RedrawEvent,
        helper_curves::{CurveCollection, RedrawCurvesEvent},
        render_info::RenderInformation,
    },
    nurbs::{
        bezier::{de_casteljau, derive_after_de_casteljau},
        parametric::Parametric,
        point::Point,
    },
    picking3d::picking_3d::Picking3dInteractable,
    projection::ProjectedSnappingDetector,
};

use super::{
    accumulated::AccumulatedMovementStore,
    proximity_detector::ProximitySnappingDetector,
    translation_controller::{
        CantSnapToCurve, CantSnapToEntities, Control, ControlParent, MovedEntityEvent,
        SnappedArrow, SnappedPoint, SnappingBehaviour, StepMode, TemporaryCurveSnappingBlocker,
        TranslationControllerState, drag_controller, drag_controller3d, draw_arrow,
    },
};

const SNAPPING_DIST: f32 = 0.07;

#[allow(clippy::complexity)]
#[derive(SystemParam)]
pub struct ObligatoryDragParams<'w, 's> {
    commands: Commands<'w, 's>,
    transform_set: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, Write<Transform>, (Without<Control>, Without<ControlParent>)>,
            CurveCollection<'w, 's>,
            ProjectedSnappingDetector<'w, 's>,
            ProximitySnappingDetector<'w, 's>,
        ),
    >,
    redraw_writer: EventWriter<'w, RedrawEvent>,
    redraw_curves_writer: EventWriter<'w, RedrawCurvesEvent>,
    snapped: Query<'w, 's, (Entity, Write<SnappedPoint>)>,
    info: Res<'w, RenderInformation>,
    state: Res<'w, TranslationControllerState>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    cant_snap_to_curve: Query<'w, 's, Read<CantSnapToCurve>>,
    cant_snap_to_entities: Query<'w, 's, Read<CantSnapToEntities>>,
    is_temporarily_blocked: Query<'w, 's, Read<TemporaryCurveSnappingBlocker>>,
    moved_entity_writer: EventWriter<'w, MovedEntityEvent>,
    accumulated_movement: ResMut<'w, AccumulatedMovementStore>,
    snapped_control_arrow: Query<'w, 's, Entity, With<SnappedArrow>>,
}

impl<'w, 's> ObligatoryDragParams<'w, 's> {
    fn handel_default(
        &mut self,
        control_parent: (Entity, &ControlParent),
        translation: Vec3,
    ) -> Vec3 {
        let mut p0 = self.transform_set.p0();
        let mut t = p0.get_mut(control_parent.1.0).unwrap();
        t.translation = match self.state.step_mode {
            StepMode::MM1 => {
                if self
                    .accumulated_movement
                    .current_diff(&control_parent.1.0)
                    .unwrap()
                    .length()
                    > 0.01 * self.info.scale
                {
                    let current = self
                        .accumulated_movement
                        .current_point(&control_parent.1.0)
                        .unwrap();
                    // 0.05
                    let scaled = current * (100.0 / self.info.scale);
                    let floored = scaled.round();
                    let final_vec = floored / (100.0 / self.info.scale);
                    self.accumulated_movement
                        .reset(control_parent.1.0, final_vec);
                    final_vec
                } else {
                    t.translation
                }
            }
            StepMode::MM5 => {
                if self
                    .accumulated_movement
                    .current_diff(&control_parent.1.0)
                    .unwrap()
                    .length()
                    > 0.05 * self.info.scale
                {
                    let current = self
                        .accumulated_movement
                        .current_point(&control_parent.1.0)
                        .unwrap();
                    // 0.05
                    let scaled = current * (20.0 / self.info.scale);
                    let floored = scaled.round();
                    let final_vec = floored / (20.0 / self.info.scale);
                    self.accumulated_movement
                        .reset(control_parent.1.0, final_vec);
                    final_vec
                } else {
                    t.translation
                }
            }
            StepMode::MM10 => {
                if self
                    .accumulated_movement
                    .current_diff(&control_parent.1.0)
                    .unwrap()
                    .length()
                    > 0.10 * self.info.scale
                {
                    let current = self
                        .accumulated_movement
                        .current_point(&control_parent.1.0)
                        .unwrap();
                    // 0.10
                    let scaled = current * (10.0 / self.info.scale);
                    let floored = scaled.round();
                    let final_vec = floored / (10.0 / self.info.scale);
                    self.accumulated_movement
                        .reset(control_parent.1.0, final_vec);
                    final_vec
                } else {
                    t.translation
                }
            }
            _ => t.translation + translation,
        };

        t.translation
    }

    /// Snapping to a projected entity. Once this function is called, SnappedPoint::ToProjection is
    /// added to the moved entity. Other than the SnappedPoint::ToCurve component, this is a softer
    /// constraint, that does not move the point when the snapped point is moved
    ///
    fn snap_to_projection(
        &mut self,
        control_parent: (Entity, &ControlParent),
        translation: Vec3,
        closest_move_direction: Vec3,
    ) -> Vec3 {
        self.commands
            .get_entity(control_parent.1.0)
            .unwrap()
            .insert(SnappedPoint::ToEntity);

        // Move to the snapped orthogonal projection
        let mut p0 = self.transform_set.p0();
        let mut t = p0.get_mut(control_parent.1.0).unwrap();

        t.translation = t.translation + translation + closest_move_direction;
        self.accumulated_movement
            .reset(control_parent.1.0, t.translation);
        t.translation
    }

    /// Try snapping to a curve when distance to the nearest curve is less than 0.5 * scale
    /// Once snapped, The SnappedPoint::ToCurve component is inserted to the moved entity.
    /// This marker component allows for moving the snapped to curve, and simultaneously move the
    /// snapped entity with it.
    fn try_snap_to_curve(
        &mut self,
        control_parent: (Entity, &ControlParent),
        t: Transform,
        translation: Vec3,
        cant_snap_to_curve: Option<CantSnapToCurve>,
        is_temporarily_blocked: bool,
    ) -> Vec3 {
        let shortest = self.transform_set.p1().collect_shortest(
            Point::from(t.translation + translation),
            &cant_snap_to_curve.unwrap_or_default(),
        );

        if let Some((curve, u, p, dist, points)) = &shortest
            && *dist < SNAPPING_DIST as f64 * self.info.scale as f64
            && !is_temporarily_blocked
        {
            let mut p0 = self.transform_set.p0();
            let mut t = p0.get_mut(control_parent.1.0).unwrap();
            t.translation = (*p).into();

            let mat = self.materials.add(StandardMaterial::from_color(YELLOW_600));
            let mat_hover = self.materials.add(StandardMaterial::from_color(YELLOW_400));

            let deriv = points.derive(&[*u], 1)[0];

            self.commands
                .get_entity(control_parent.1.0)
                .unwrap()
                .insert(SnappedPoint::ToCurve {
                    u: *u,
                    curve: *curve,
                });

            // Create a new arrow (directional), that follows the curvature of the
            // curve that the point was snapped to. The direction of the arrow is
            // updated each frame, to adhere to movement along the curve
            self.commands
                .get_entity(control_parent.0)
                .unwrap()
                .with_children(|cmd| {
                    cmd.spawn((
                        Control(Vec3::from(deriv)),
                        SnappedArrow,
                        Transform::default().looking_to(Vec3::from(deriv), Vec3::Y),
                        Visibility::Inherited,
                    ))
                    .with_children(|cmd| {
                        draw_arrow(
                            cmd,
                            mat,
                            mat_hover,
                            &mut self.meshes,
                            self.info.scale,
                            false,
                            Picking3dInteractable::Default,
                            Pickable::default(),
                        );
                    })
                    .observe(drag_controller)
                    .observe(drag_controller3d);
                });
            self.accumulated_movement
                .reset(control_parent.1.0, t.translation);
            t.translation
        } else {
            if let Some((_, _, _, dist, _)) = &shortest
                && *dist > 2.0 * SNAPPING_DIST as f64 * self.info.scale as f64
                && is_temporarily_blocked
            {
                let _ = self.commands.get_entity(control_parent.1.0).map(|mut e| {
                    e.remove::<TemporaryCurveSnappingBlocker>();
                });
            } else if shortest.is_none() {
                let _ = self.commands.get_entity(control_parent.1.0).map(|mut e| {
                    e.remove::<TemporaryCurveSnappingBlocker>();
                });
            }
            self.handel_default(control_parent, translation)
        }
    }

    fn try_create_snapping(
        &mut self,
        control_parent: (Entity, &ControlParent),
        t: Transform,
        translation: Vec3,
        cant_snap_to_curve: Option<CantSnapToCurve>,
        cant_snap_to_entities: Option<CantSnapToEntities>,
        is_temporarily_blocked: bool,
    ) -> Vec3 {
        // First test orthographic snap, then non orthographic snap, and last but not least, test
        // curve snapping
        if let Some(closest_move_direction) = self.transform_set.p2().detect_closest_projected(
            control_parent.1.0,
            translation,
            &cant_snap_to_entities,
        ) && closest_move_direction.length() < SNAPPING_DIST * self.info.scale
        {
            self.snap_to_projection(control_parent, translation, closest_move_direction)
        } else if let Some(closest_move_direction) =
            self.transform_set.p3().detect_closest_snappable_entity(
                control_parent.1.0,
                translation,
                &cant_snap_to_entities,
            )
            && closest_move_direction.length() < SNAPPING_DIST * self.info.scale
        {
            self.snap_to_projection(control_parent, translation, closest_move_direction)
        } else {
            self.try_snap_to_curve(
                control_parent,
                t,
                translation,
                cant_snap_to_curve,
                is_temporarily_blocked,
            )
        }
    }

    #[allow(clippy::complexity)]
    fn check_and_process_already_snapped(
        &mut self,
        control_parent: (Entity, &ControlParent),
        t: Transform,
        translation: Vec3,
        cant_snap_to_curve: Option<CantSnapToCurve>,
        cant_snap_to_entities: Option<CantSnapToEntities>,
        snap: SnappedPoint,
        is_snapped_arrow: bool,
        is_temporarily_blocked: bool,
    ) -> Vec3 {
        match snap {
            SnappedPoint::ToCurve { u: _, curve: _ } => {
                let point = if is_snapped_arrow {
                    Point::from(t.translation + translation)
                } else {
                    Point::from(
                        self.accumulated_movement
                            .current_point(&control_parent.1.0)
                            .unwrap(),
                    )
                };

                let shortest = self
                    .transform_set
                    .p1()
                    .collect_shortest(point, &cant_snap_to_curve.unwrap_or_default());
                if let Some((curve, u, p, dist, _)) = shortest
                    && dist < SNAPPING_DIST as f64 * self.info.scale as f64
                {
                    if is_snapped_arrow {
                        let mut snap = self.snapped.get_mut(control_parent.1.0).unwrap().1;
                        *snap = SnappedPoint::ToCurve { u, curve };

                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();

                        t.translation = p.into();
                        self.accumulated_movement
                            .reset(control_parent.1.0, t.translation);
                        t.translation
                    } else {
                        t.translation
                    }
                } else {
                    let entity = self.snapped.get(control_parent.1.0).unwrap().0;
                    self.commands
                        .get_entity(entity)
                        .unwrap()
                        .remove::<SnappedPoint>()
                        .insert(TemporaryCurveSnappingBlocker);

                    // Once removed (Snapped point, consider adding it back when snapping to a
                    // projected position)
                    if let Some(closest_move_direction) =
                        self.transform_set.p2().detect_closest_projected(
                            control_parent.1.0,
                            translation,
                            &cant_snap_to_entities,
                        )
                        && closest_move_direction.length() < SNAPPING_DIST * self.info.scale
                    {
                        let _ = self.commands.get_entity(entity).map(|mut e| {
                            e.remove::<TemporaryCurveSnappingBlocker>();
                        });
                        self.snap_to_projection(control_parent, translation, closest_move_direction)
                    } else if let Some(closest_move_direction) =
                        self.transform_set.p3().detect_closest_snappable_entity(
                            control_parent.1.0,
                            translation,
                            &cant_snap_to_entities,
                        )
                        && closest_move_direction.length() < SNAPPING_DIST * self.info.scale
                    {
                        let _ = self.commands.get_entity(entity).map(|mut e| {
                            e.remove::<TemporaryCurveSnappingBlocker>();
                        });
                        self.snap_to_projection(control_parent, translation, closest_move_direction)
                    } else {
                        self.handel_default(control_parent, translation)
                    }
                }
            }
            SnappedPoint::ToEntity => {
                // Once removed (Snapped point, consider adding it back when snapping to a
                // projected position)
                if let Some(closest_move_direction) =
                    self.transform_set.p2().detect_closest_projected(
                        control_parent.1.0,
                        self.accumulated_movement
                            .current_diff(&control_parent.1.0)
                            .unwrap(),
                        &cant_snap_to_entities,
                    )
                    && closest_move_direction.length() < SNAPPING_DIST * self.info.scale
                {
                    let mut p0 = self.transform_set.p0();
                    let mut t = p0.get_mut(control_parent.1.0).unwrap();
                    t.translation = t.translation
                        + self
                            .accumulated_movement
                            .current_diff(&control_parent.1.0)
                            .unwrap()
                        + closest_move_direction;
                    t.translation
                } else if let Some(closest_move_direction) =
                    self.transform_set.p3().detect_closest_snappable_entity(
                        control_parent.1.0,
                        self.accumulated_movement
                            .current_diff(&control_parent.1.0)
                            .unwrap(),
                        &cant_snap_to_entities,
                    )
                    && closest_move_direction.length() < SNAPPING_DIST * self.info.scale
                {
                    let mut p0 = self.transform_set.p0();
                    let mut t = p0.get_mut(control_parent.1.0).unwrap();
                    t.translation = t.translation
                        + self
                            .accumulated_movement
                            .current_diff(&control_parent.1.0)
                            .unwrap()
                        + closest_move_direction;
                    t.translation
                } else {
                    // Remove snapped component
                    let entity = self.snapped.get(control_parent.1.0).unwrap().0;
                    self.commands
                        .get_entity(entity)
                        .unwrap()
                        .remove::<SnappedPoint>();

                    // Consider adding back the curve when not
                    if let Some(accumulated_diff) =
                        self.accumulated_movement.current_diff(&control_parent.1.0)
                    {
                        self.try_snap_to_curve(
                            control_parent,
                            t,
                            accumulated_diff,
                            cant_snap_to_curve,
                            is_temporarily_blocked,
                        )
                    } else {
                        t.translation
                    }
                }
            }
        }
    }

    pub fn update_position_drag_universal(
        &mut self,
        control_parent: (Entity, &ControlParent),
        translation: Vec3,
        arrow: Entity,
    ) {
        if self
            .accumulated_movement
            .current_diff(&control_parent.1.0)
            .is_none()
        {
            self.accumulated_movement.start_movement_entity(
                control_parent.1.0,
                self.transform_set
                    .p0()
                    .get(control_parent.1.0)
                    .unwrap()
                    .translation,
            );
        }
        self.accumulated_movement
            .add_diff(&control_parent.1.0, translation);
        let is_snapped_arrow = self.snapped_control_arrow.get(arrow).is_ok();
        // Only adjust the control parent
        let is_already_snapped = self.snapped.get(control_parent.1.0).is_ok();
        let cant_snap_to_curve = self
            .cant_snap_to_curve
            .get(control_parent.1.0)
            .ok()
            .cloned();

        let cant_snap_to_entities = self
            .cant_snap_to_entities
            .get(control_parent.1.0)
            .ok()
            .cloned();

        let is_temporarily_blocked = self.is_temporarily_blocked.get(control_parent.1.0).is_ok();

        let changed_entity = control_parent.1.0;
        let control_point = self.transform_set.p0().get(control_parent.1.0).copied();
        if let Ok(t) = control_point {
            if t.translation.is_nan() {
                panic!("T is none");
            }
            let started_translation = t.translation;

            let ending_translation = if self.state.curve_snapping == SnappingBehaviour::Snap {
                if !is_already_snapped {
                    self.try_create_snapping(
                        control_parent,
                        t,
                        translation,
                        cant_snap_to_curve,
                        cant_snap_to_entities,
                        is_temporarily_blocked,
                    )
                } else {
                    // The entity is already snapped to a curve. Either continue snapping by moving
                    // the entity back to the curve, or if the distance is to large, remove the
                    // snapping
                    let snap = self
                        .snapped
                        .get(control_parent.1.0)
                        .ok()
                        .map(|v| *v.1)
                        .expect(
                            "This is already checked in this branch by is_already_snapped == true",
                        );

                    self.check_and_process_already_snapped(
                        control_parent,
                        t,
                        translation,
                        cant_snap_to_curve,
                        cant_snap_to_entities,
                        snap,
                        is_snapped_arrow,
                        is_temporarily_blocked,
                    )
                }
            } else {
                // Always remove the snapped point, when snapping behaviour is off
                self.commands
                    .entity(control_parent.1.0)
                    .remove::<SnappedPoint>();

                let mut p0 = self.transform_set.p0();
                let mut t = p0.get_mut(control_parent.1.0).unwrap();
                t.translation += translation;
                t.translation
            };

            // Trigger the moved event, so that linked entities may update the position of the
            // linked entity, correspondingly (using lokal transforms)
            if let Ok(mut entity) = self.commands.get_entity(changed_entity) {
                let delta = ending_translation - started_translation;
                entity.trigger(MovedEntityEvent {
                    entity: changed_entity,
                    delta,
                });
                self.moved_entity_writer.write(MovedEntityEvent {
                    entity: changed_entity,
                    delta,
                });
            }
        }

        self.redraw_writer.write(RedrawEvent::Fast);
        self.redraw_curves_writer.write(RedrawCurvesEvent);
    }
}

//! Drag models of your own, flown in place of hpr's drag buildup.
//!
//! A [`DragModel`] gives the whole rocket's zero-lift drag coefficient `C_D0` at a flow, on the
//! rocket's reference area ([`AeroModel::reference_area_m2`]). [`AeroModel::with_drag_model`]
//! puts one in place of the buildup, and of any drag table ([`crate::table`]); a flight takes one
//! through `hpr_sim::Simulation::with_drag_model`.
//!
//! The model replaces the zero-lift drag only, as a drag table does. The rest stays hpr's:
//!
//! - At an angle of attack the axial coefficient is `C_A = C_D0 f(α)`, with hpr's
//!   [`crate::drag::axial_drag_alpha_factor`].
//! - The normal force, the centre of pressure, the roll and the pitch and yaw damping are hpr's
//!   own, from [`AeroModel::normal_force`] and [`AeroModel::roll`].
//!
//! A model is asked a [`DragQuery`]: the flow (Mach number and angles), the drag conditions
//! (Reynolds number per metre, whether a motor is thrusting) and hpr's own buildup at that flow,
//! so a model can adjust hpr's number instead of replacing it.
//!
//! **How far to trust it:** as far as the model. hpr checks only that the coefficient it returns
//! is finite and not negative; it can't know whether the number is right.
//!
//! ```
//! use hpr_aero::{AeroError, AeroModel, DragConditions, DragModel, DragQuery, Flow};
//!
//! /// hpr's own drag, 10% higher: a rougher finish than the design says, say.
//! #[derive(Debug)]
//! struct TenPercentMore;
//!
//! impl DragModel for TenPercentMore {
//!     fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
//!         Ok(1.1 * query.buildup()?.zero_lift_coefficient)
//!     }
//! }
//!
//! let rocket: hpr_design::Rocket = serde_json::from_str(include_str!(
//!     "../../../validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json"
//! ))?;
//! let hpr = AeroModel::new(&rocket.layout()?)?;
//! let custom = hpr.clone().with_drag_model(TenPercentMore);
//!
//! let (flow, conditions) = (Flow::axial(0.3), DragConditions::coasting(6.0e6));
//! let own = hpr.drag(&flow, &conditions)?.zero_lift_coefficient;
//! let more = custom.drag(&flow, &conditions)?.zero_lift_coefficient;
//! assert!((more - 1.1 * own).abs() < 1e-15);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::fmt;
use std::sync::Arc;

use crate::drag::{Drag, DragConditions};
use crate::error::AeroError;
use crate::model::{AeroModel, Flow};

/// A rocket's zero-lift drag, given by a program in place of hpr's drag buildup.
///
/// Implement [`DragModel::zero_lift_drag`] and hand the model to
/// [`AeroModel::with_drag_model`] or `hpr_sim::Simulation::with_drag_model`. The flight asks it
/// at every evaluation of the equations of motion, several times a step, so it should be quick
/// and give the same answer to the same question: a flight's determinism is only as good as its
/// model's.
///
/// A model is shared, not copied: it is kept behind an [`Arc`], so a model that holds a large
/// table costs nothing to fly many times, and an `Arc<dyn DragModel>` is itself a model.
pub trait DragModel: fmt::Debug + Send + Sync {
    /// The whole rocket's zero-lift drag coefficient `C_D0` at `query`'s flow, on the rocket's
    /// reference area ([`DragQuery::reference_area_m2`]).
    ///
    /// # Errors
    ///
    /// Whatever the model can't answer, such as a Mach number past its data. An
    /// [`AeroError`] from [`DragQuery::buildup`] can be passed on as it is.
    fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError>;
}

impl<T: DragModel + ?Sized> DragModel for Arc<T> {
    fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
        (**self).zero_lift_drag(query)
    }
}

/// What a [`DragModel`] is asked: the flow, the drag conditions, and the rocket's own model.
#[derive(Debug, Clone, Copy)]
pub struct DragQuery<'a> {
    flow: &'a Flow,
    conditions: &'a DragConditions,
    model: &'a AeroModel,
}

impl<'a> DragQuery<'a> {
    /// A question about `model`'s rocket at `flow` and `conditions`, for trying a
    /// [`DragModel`] on its own; a flight makes its own.
    pub fn new(flow: &'a Flow, conditions: &'a DragConditions, model: &'a AeroModel) -> Self {
        Self {
            flow,
            conditions,
            model,
        }
    }

    /// The flow: Mach number, angle of attack and roll. The coefficient asked for is the
    /// zero-lift one whatever the angle of attack; hpr scales it for the angle itself.
    pub fn flow(&self) -> &Flow {
        self.flow
    }

    /// The Mach number, as a shorthand for `flow().mach`: finite and not negative.
    pub fn mach(&self) -> f64 {
        self.flow.mach
    }

    /// The drag conditions: the Reynolds number per metre, and whether a motor is thrusting,
    /// which a model with a power-on curve reads.
    pub fn conditions(&self) -> &DragConditions {
        self.conditions
    }

    /// The area the coefficient is on: the rocket's reference area, m².
    pub fn reference_area_m2(&self) -> f64 {
        self.model.reference_area_m2()
    }

    /// The rocket's own aerodynamic model, for what else a drag model needs of it, such as its
    /// length ([`AeroModel::length_m`]) or its drag buildup by component
    /// ([`AeroModel::buildup_components`]).
    pub fn model(&self) -> &AeroModel {
        self.model
    }

    /// hpr's own drag buildup at this flow and these conditions ([`AeroModel::buildup_drag`]):
    /// what the flight would have used without the model.
    ///
    /// # Errors
    ///
    /// As [`AeroModel::buildup_drag`]: among others, [`AeroError::Mach`] at Mach 5 and faster,
    /// where the buildup ends.
    pub fn buildup(&self) -> Result<Drag, AeroError> {
        self.model.buildup_drag(self.flow, self.conditions)
    }
}

/// A [`DragModel`] held by an [`AeroModel`]. Two are equal when they are the same model, so an
/// [`AeroModel`] and its clone stay equal.
#[derive(Clone)]
pub(crate) struct SharedDragModel(pub(crate) Arc<dyn DragModel>);

impl PartialEq for SharedDragModel {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for SharedDragModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::table::DragTable;
    use crate::testing::finned_rocket;

    fn model() -> AeroModel {
        AeroModel::new(&finned_rocket(3).layout().unwrap()).unwrap()
    }

    /// hpr's own buildup, handed back unchanged.
    #[derive(Debug)]
    struct Buildup;

    impl DragModel for Buildup {
        fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
            Ok(query.buildup()?.zero_lift_coefficient)
        }
    }

    /// The same coefficient at every flow.
    #[derive(Debug)]
    struct Constant(f64);

    impl DragModel for Constant {
        fn zero_lift_drag(&self, _query: &DragQuery<'_>) -> Result<f64, AeroError> {
            Ok(self.0)
        }
    }

    /// Keeps the conditions it was asked with.
    #[derive(Debug, Default)]
    struct Spy(Mutex<Vec<DragConditions>>);

    impl DragModel for Spy {
        fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
            self.0.lock().unwrap().push(*query.conditions());
            Ok(0.5)
        }
    }

    #[test]
    fn a_model_handing_back_the_buildup_changes_nothing() {
        let own = model();
        let custom = own.clone().with_drag_model(Buildup);
        for mach in [0.0, 0.3, 0.95, 1.2, 2.5, 4.9] {
            for alpha_deg in [0.0, 4.0, 17.0, 60.0, 120.0] {
                let flow = Flow::new(mach, f64::to_radians(alpha_deg), 0.3);
                for conditions in [
                    DragConditions::coasting(5.0e6),
                    DragConditions::thrusting(5.0e6, 0.0005),
                ] {
                    assert_eq!(
                        custom.drag(&flow, &conditions).unwrap(),
                        Drag {
                            friction: 0.0,
                            pressure: 0.0,
                            base: 0.0,
                            parasitic: 0.0,
                            ..own.drag(&flow, &conditions).unwrap()
                        },
                        "Mach {mach}, {alpha_deg}°"
                    );
                    assert_eq!(
                        custom.buildup_drag(&flow, &conditions).unwrap(),
                        own.drag(&flow, &conditions).unwrap()
                    );
                }
            }
        }
        // The buildup ends at Mach 5; the model passes its refusal on.
        let past = Flow::axial(5.0);
        let conditions = DragConditions::coasting(5.0e6);
        assert!(matches!(
            custom.drag(&past, &conditions),
            Err(AeroError::Mach { .. })
        ));
    }

    #[test]
    fn a_constant_model_flies_as_a_constant_table() {
        let table = DragTable::from_csv("0,0.45\n1,0.45\n", None).unwrap();
        let with_table = model().with_drag_table(table);
        let with_model = model().with_drag_model(Constant(0.45));
        for mach in [0.0, 0.5, 3.0, 7.0] {
            for alpha_deg in [0.0, 10.0, 170.0] {
                let flow = Flow::new(mach, f64::to_radians(alpha_deg), 0.0);
                let conditions = DragConditions::coasting(1.0e6);
                let from_table = with_table.drag(&flow, &conditions).unwrap();
                let from_model = with_model.drag(&flow, &conditions).unwrap();
                assert_eq!(
                    from_model.zero_lift_coefficient,
                    from_table.zero_lift_coefficient
                );
                assert_eq!(from_model.axial_coefficient, from_table.axial_coefficient);
                assert!(from_model.table.is_none());
            }
        }
    }

    #[test]
    fn a_model_and_a_table_replace_each_other() {
        let table = DragTable::from_csv("0,0.45\n1,0.45\n", None).unwrap();
        let model_last = model()
            .with_drag_table(table.clone())
            .with_drag_model(Constant(0.7));
        assert!(model_last.drag_table().is_none() && model_last.drag_model().is_some());
        let table_last = model()
            .with_drag_model(Constant(0.7))
            .with_drag_table(table);
        assert!(table_last.drag_model().is_none() && table_last.drag_table().is_some());
        let conditions = DragConditions::coasting(1.0e6);
        let flow = Flow::axial(0.5);
        let drag = |m: &AeroModel| m.drag(&flow, &conditions).unwrap().zero_lift_coefficient;
        assert_eq!(drag(&model_last), 0.7);
        assert_eq!(drag(&table_last), 0.45);
    }

    #[test]
    fn a_model_is_asked_the_conditions_the_buildup_reads() {
        let spy = Arc::new(Spy::default());
        let asked = || spy.0.lock().unwrap().pop().unwrap();
        let conditions = DragConditions::thrusting(1.0e6, 0.0005).with_pod_motors(0.0002);
        let flow = Flow::axial(0.5);
        model()
            .with_drag_model(Arc::clone(&spy))
            .drag(&flow, &conditions)
            .unwrap();
        assert_eq!(asked(), conditions);
        // With the base kept whole, the motors' areas are read as zero, for the model as for the
        // buildup; it still hears that a motor burns.
        model()
            .with_full_base_drag_under_power()
            .with_drag_model(Arc::clone(&spy))
            .drag(&flow, &conditions)
            .unwrap();
        let read = asked();
        assert!(read.thrusting);
        assert_eq!(read.thrusting_motor_area_m2, 0.0);
        assert_eq!(read.thrusting_pod_motor_area_m2, 0.0);
    }

    #[test]
    fn a_coefficient_that_is_negative_or_not_finite_is_refused() {
        let conditions = DragConditions::coasting(1.0e6);
        let flow = Flow::axial(0.5);
        for bad in [-0.01, f64::NAN, f64::INFINITY] {
            let error = model()
                .with_drag_model(Constant(bad))
                .drag(&flow, &conditions)
                .unwrap_err();
            assert!(
                matches!(error, AeroError::Domain { what, value }
                    if what == "zero-lift drag coefficient from a drag model"
                        && value.to_bits() == bad.to_bits()),
                "{error:?}"
            );
        }
        // Zero is a coefficient, if an odd one.
        let zero = model().with_drag_model(Constant(0.0));
        assert_eq!(
            zero.drag(&flow, &conditions).unwrap().axial_coefficient,
            0.0
        );
        // A Mach number the model can't be asked about, and angles out of their domain.
        let custom = model().with_drag_model(Constant(0.5));
        for mach in [-0.1, f64::NAN] {
            let error = custom.drag(&Flow::axial(mach), &conditions).unwrap_err();
            assert!(
                matches!(error, AeroError::Domain { what, .. }
                    if what == "Mach number for a drag model"),
                "{error:?}"
            );
        }
        assert!(
            custom
                .drag(&Flow::new(0.5, -0.1, 0.0), &conditions)
                .is_err()
        );
    }

    #[test]
    fn a_models_own_error_is_passed_on() {
        #[derive(Debug)]
        struct Refuses;
        impl DragModel for Refuses {
            fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
                Err(AeroError::Domain {
                    what: "Mach number past the model's data",
                    value: query.mach(),
                })
            }
        }
        let error = model()
            .with_drag_model(Refuses)
            .drag(&Flow::axial(0.5), &DragConditions::coasting(1.0e6))
            .unwrap_err();
        assert!(
            matches!(error, AeroError::Domain { what, value }
                if what == "Mach number past the model's data" && value == 0.5),
            "{error:?}"
        );
    }

    #[test]
    fn a_clone_is_equal_and_another_model_is_not() {
        let custom = model().with_drag_model(Constant(0.5));
        assert_eq!(custom.clone(), custom);
        assert_ne!(model().with_drag_model(Constant(0.5)), custom);
        assert_ne!(model(), custom);
        let query_model = model();
        let (flow, conditions) = (Flow::axial(0.2), DragConditions::coasting(1.0e6));
        let query = DragQuery::new(&flow, &conditions, &query_model);
        assert_eq!(query.mach(), 0.2);
        assert_eq!(query.reference_area_m2(), query_model.reference_area_m2());
        assert_eq!(*query.flow(), flow);
        assert_eq!(query.model(), &query_model);
    }
}

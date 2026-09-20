use crate::{Control, State};

pub trait TransitionModel {
    fn f(&self, state: &State, control: Option<&Control>) -> State;
}

#[derive(Clone, Debug, Default)]
pub struct DefaultTransition;

impl TransitionModel for DefaultTransition {
    fn f(&self, state: &State, _control: Option<&Control>) -> State {
        let x = state.as_vector();
        let dt = 1.0;

        let new_capacity = (x[0] + dt * x[1]).max(0.0);
        let new_investment = (x[1] + dt * x[4].mul_add(-0.05, x[2] * 0.1)).max(0.0);
        let new_labour = (x[2] + dt * (x[3] * 0.2)).max(0.0);
        let new_fiscal = (x[3] + dt * (x[0] * 0.05)).max(0.0);
        let new_demand = (x[4] + dt * x[6].mul_add(-0.02, x[0] * 0.03)).max(0.0);
        let new_housing = (x[5] + dt * (x[0] * 0.02)).max(0.0);
        let new_geopolitical = (x[6] + dt * 0.01).max(0.0);
        let new_migration = (x[7] + dt * (x[2] * 0.05)).max(0.0);

        State::new(
            new_capacity,
            new_investment,
            new_labour,
            new_fiscal,
            new_demand,
            new_housing,
            new_geopolitical,
            new_migration,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::State;

    #[test]
    fn default_transition_preserves_non_negative_state() {
        let transition = DefaultTransition;
        let state = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let next = transition.f(&state, None);
        assert!(next.capacity() >= 0.0);
        assert!(next.investment() >= 0.0);
        assert!(next.fiscal_capacity() >= 0.0);
    }

    #[test]
    fn default_transition_changes_state() {
        let transition = DefaultTransition;
        let state = State::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let next = transition.f(&state, None);
        assert_ne!(next.as_vector(), state.as_vector());
    }

    #[test]
    fn default_transition_ignores_control_input() {
        let transition = DefaultTransition;
        let state = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let control = Some(crate::Control::from_vec(vec![0.0; 8]));
        let next = transition.f(&state, control.as_ref());
        assert!(next.capacity().is_finite());
    }
}

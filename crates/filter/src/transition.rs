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

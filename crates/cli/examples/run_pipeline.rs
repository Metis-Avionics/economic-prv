#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::too_many_lines,
    clippy::used_underscore_binding,
    clippy::expect_used,
    clippy::use_debug,
    clippy::let_underscore_must_use
)]

use nalgebra::{DMatrix, SMatrix};
use prv_core::{Observation, State, TimeSeries};
use prv_data::DataLoader;
use prv_evaluation::{Evaluator, Model};
use prv_filter::{Ekf, observation::ObservationModel, transition::DefaultTransition};
use prv_monte_carlo::{ShockSpec, ShockType, Simulator};
use prv_policy::{PolicyEngine, PolicyWeights, Regime};

struct LinearObservationModel {
    h: DMatrix<f64>,
    r: DMatrix<f64>,
}

impl ObservationModel for LinearObservationModel {
    fn h(&self, state: &State) -> Observation {
        let vec = &self.h * state.as_vector();
        Observation::new(vec.as_slice().to_vec())
    }

    fn jacobian_h(&self, _state: &State) -> DMatrix<f64> {
        self.h.clone()
    }

    fn r(&self) -> DMatrix<f64> {
        self.r.clone()
    }
}

struct NaiveRegime;

impl Model for NaiveRegime {
    fn predict(&self, state: &State) -> State {
        state.clone()
    }
}

fn main() {
    println!("=== PRV Full Pipeline Demo ===\n");

    let loader = DataLoader::new();
    let df = loader
        .load_historical("../../examples/faux_data.csv")
        .expect("load faux_data.csv");
    println!(
        "Loaded DataFrame: {} rows x {} columns",
        df.row_count,
        df.columns.len()
    );
    println!("Columns: {:?}\n", df.columns);

    let observations = loader
        .to_observations(&df)
        .expect("convert to observations");
    println!("Generated {} observations\n", observations.len());

    let mut ekf = Ekf::new(
        DefaultTransition,
        LinearObservationModel {
            h: DMatrix::<f64>::identity(10, 8),
            r: DMatrix::<f64>::identity(10, 10) * 0.1,
        },
        State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
        SMatrix::<f64, 8, 8>::identity() * 0.5,
        SMatrix::<f64, 8, 8>::identity() * 0.01,
        DMatrix::<f64>::identity(10, 10) * 0.1,
    );

    println!("Running EKF on observations...");
    for obs in &observations {
        let _ = ekf.predict(None);
        let _ = ekf.update(obs);
    }
    println!("Final EKF state: {:?}\n", ekf.x_hat.as_vector().transpose());

    let mean = ekf.x_hat;
    let covariance = SMatrix::<f64, 8, 8>::identity() * 0.2;
    let shocks = vec![
        ShockSpec {
            shock_type: ShockType::Demand,
            amplitude: 0.05,
            persistence: 0.8,
            autocorrelation: 0.1,
        },
        ShockSpec {
            shock_type: ShockType::Geopolitical,
            amplitude: 0.03,
            persistence: 0.9,
            autocorrelation: 0.05,
        },
    ];

    println!("Running Monte Carlo simulation...");
    let mc = Simulator::new(42)
        .simulate(&mean, &covariance, 100, 8, &shocks)
        .expect("Monte Carlo simulation");
    println!(
        "Monte Carlo mean state: {:?}\n",
        mc.mean.as_vector().transpose()
    );

    println!("Evaluating policy...");
    let engine = PolicyEngine::new(PolicyWeights::default());
    let policy = engine.evaluate(&mc, &Regime::Expansion, Some(42));
    println!(
        "Policy recommended actions: {:?}\n",
        policy.recommended_action_distribution
    );

    println!("Running backtest evaluation...");
    let evaluator = Evaluator::new();
    let ts = TimeSeries {
        timestamps: vec![],
        values: observations
            .iter()
            .map(|_o| State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0))
            .collect(),
        frequency: Some("quarterly".to_string()),
    };
    let results = evaluator.backtest(&NaiveRegime, &ts, 4);
    println!(
        "Backtest metrics: RMSE={:.4}, MAE={:.4}\n",
        results.metrics.rmse, results.metrics.mae
    );

    println!("=== Pipeline complete ===");
}

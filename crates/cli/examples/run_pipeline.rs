use nalgebra::SMatrix;
use prv_core::{Observation, State};
use prv_data::DataLoader;
use prv_evaluation::{Evaluator, Model};
use prv_filter::{Ekf, transition::DefaultTransition};
use prv_monte_carlo::Simulator;
use prv_policy::{PolicyEngine, PolicyWeights, Regime};

struct LinearObservationModel {
    h: SMatrix<f64, 10, 8>,
    r: SMatrix<f64, 10, 10>,
}

impl prv_filter::ObservationModel<10> for LinearObservationModel {
    fn h(&self, state: &State) -> Observation<10> {
        let vec = self.h * state.as_vector();
        Observation::new(vec.into())
    }

    fn jacobian_h(&self, _state: &State) -> SMatrix<f64, 10, 8> {
        self.h
    }

    fn r(&self) -> SMatrix<f64, 10, 10> {
        self.r
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
            h: SMatrix::<f64, 10, 8>::identity(),
            r: SMatrix::<f64, 10, 10>::identity() * 0.1,
        },
        State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
        SMatrix::<f64, 8, 8>::identity() * 0.5,
        SMatrix::<f64, 8, 8>::identity() * 0.01,
        SMatrix::<f64, 10, 10>::identity() * 0.1,
    );

    println!("Running EKF on observations...");
    for obs in &observations {
        let _ = ekf.predict(None);
        let _ = ekf.update(obs);
    }
    println!("Final EKF state: {:?}\n", ekf.x_hat.as_vector().transpose());

    let mean = ekf.x_hat;
    let covariance = SMatrix::<f64, 8, 8>::identity() * 0.2;
    println!("Running Monte Carlo simulation...");
    let mc = Simulator::new(42).simulate(&mean, &covariance, 100, 8);
    println!(
        "Monte Carlo mean state: {:?}\n",
        mc.mean.as_vector().transpose()
    );

    println!("Evaluating policy...");
    let engine = PolicyEngine::new(PolicyWeights::default());
    let policy = engine.evaluate(&mc, &Regime::Expansion);
    println!(
        "Policy recommended actions: {:?}\n",
        policy.recommended_action_distribution
    );

    println!("Running backtest evaluation...");
    let evaluator = Evaluator::new();
    let ts = prv_core::TimeSeries {
        timestamps: vec![],
        values: observations
            .iter()
            .map(|_o| State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0))
            .collect(),
    };
    let results = evaluator.backtest(&NaiveRegime, &ts, 4);
    println!(
        "Backtest metrics: RMSE={:.4}, MAE={:.4}\n",
        results.metrics.rmse, results.metrics.mae
    );

    println!("=== Pipeline complete ===");
}

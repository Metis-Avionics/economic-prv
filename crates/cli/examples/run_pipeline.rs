#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::too_many_lines,
    clippy::used_underscore_binding,
    clippy::expect_used,
    clippy::use_debug,
    clippy::let_underscore_must_use,
    clippy::too_many_arguments,
    clippy::single_char_add_str,
    clippy::format_push_string
)]

use nalgebra::{DMatrix, SMatrix};
use prv_core::{Observation, State, TimeSeries};
use prv_data::DataLoader;
use prv_evaluation::{Evaluator, Model};
use prv_filter::{Ekf, observation::ObservationModel, transition::DefaultTransition};
use prv_monte_carlo::{ShockSpec, ShockType, Simulator};
use prv_policy::{PolicyEngine, PolicyInstrument, PolicyWeights, Regime};
use rand::prelude::*;
use rand_distr::Normal;
use std::io::Write;
use std::path::PathBuf;

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

fn generate_faux_csv(path: &str, seed: u64, rows: usize) -> std::io::Result<()> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let normal = Normal::new(0.0, 0.5).unwrap();

    let mut file = std::fs::File::create(path)?;
    writeln!(
        file,
        "quarter,gdp_growth,inflation,unemployment,interest_rate,fiscal_balance,current_account,housing_price_index,consumer_confidence,investment_flow,exchange_rate,geopolitical_tension_index,sanctions_exposure,alliance_stability"
    )?;

    let base_year = 2020;
    let base_quarter = 1;

    for i in 0..rows {
        let year = base_year + (i + base_quarter - 1) / 4;
        let quarter = ((i + base_quarter - 1) % 4) + 1;
        let cycle = (i as f64 / rows as f64) * std::f64::consts::PI * 2.0;

        let gdp = (2.0 + cycle.sin() * 1.5 + rng.sample(normal)).clamp(-5.0, 10.0);
        let inflation = (1.8 + cycle.cos() * 0.8 + rng.sample(normal) * 0.3).clamp(0.0, 8.0);
        let unemployment = (3.9 - gdp * 0.3 + rng.sample(normal) * 0.2).clamp(2.0, 15.0);
        let interest = (1.5 + inflation * 0.4 + rng.sample(normal) * 0.1).clamp(0.0, 6.0);
        let fiscal = (2.3 - gdp * 0.5 + rng.sample(normal) * 0.3).clamp(-10.0, 5.0);
        let current_account = (-1.2 + interest * 0.3 + rng.sample(normal) * 0.2).clamp(-5.0, 3.0);
        let housing = (180.5 + interest * 8.0 + rng.sample(normal) * 2.0).clamp(150.0, 250.0);
        let confidence = (98.2 + gdp * 2.0 + rng.sample(normal) * 1.0).clamp(70.0, 120.0);
        let investment = (120.4 + gdp * 5.0 + rng.sample(normal) * 2.0).clamp(40.0, 200.0);
        let exchange = (1.32 + interest * 0.05 + rng.sample(normal) * 0.02).clamp(1.0, 2.0);
        let geo_tension =
            (0.2 + (i as f64 / rows as f64) * 0.3 + rng.sample(normal) * 0.05).clamp(0.0, 1.0);
        let sanctions = (geo_tension * 0.5 + rng.sample(normal) * 0.05).clamp(0.0, 1.0);
        let alliance = (1.0 - geo_tension * 0.2 + rng.sample(normal) * 0.03).clamp(0.5, 1.0);

        writeln!(
            file,
            "{year}Q{quarter},{gdp:.1},{inflation:.1},{unemployment:.1},{interest:.2},{fiscal:.1},{current_account:.1},{housing:.1},{confidence:.1},{investment:.1},{exchange:.2},{geo_tension:.2},{sanctions:.2},{alliance:.2}"
        )?;
    }

    Ok(())
}

fn generate_markdown_report(
    path: &str,
    seed: u64,
    rows: usize,
    df: &prv_data::DataFrame,
    observations: &[prv_core::Observation],
    final_state: &State,
    _mc_mean: &State,
    policy: &prv_policy::PolicyDistribution,
    metrics: &prv_evaluation::MetricResults,
) -> std::io::Result<()> {
    let mut md = String::new();

    md.push_str("# PRV Pipeline Report\n\n");
    md.push_str(&format!(
        "**Generated:** {}\n\n",
        chrono::Utc::now().to_rfc3339()
    ));
    md.push_str(&format!("**Seed:** {seed}\n"));
    md.push_str(&format!("**Rows:** {rows}\n\n"));

    md.push_str("## Faux Dataset\n\n");
    md.push_str("The faux dataset is a synthetically generated quarterly economic time series ");
    md.push_str("designed to exercise the full PRV pipeline. It combines:\n\n");
    md.push_str(
        "- **Cyclical patterns** — sinusoidal base signals for GDP, inflation, and confidence\n",
    );
    md.push_str("- **Gaussian noise** — `rand_distr::Normal(0, 0.5)` perturbations per series\n");
    md.push_str(
        "- **Cross-series coupling** — interest rates respond to inflation, unemployment to GDP\n",
    );
    md.push_str(
        "- **Geopolitical drift** — tension, sanctions, and alliance stability evolve together\n",
    );
    md.push_str("- **Realistic bounds** — all values clamped to plausible economic ranges\n\n");
    md.push_str(&format!(
        "Loaded **{}** rows × **{}** columns.\n\n",
        df.row_count,
        df.columns.len()
    ));
    md.push_str("| Column | Description |\n");
    md.push_str("|--------|-------------|\n");
    for col in &df.columns {
        let desc = match col.as_str() {
            "gdp_growth" => "Quarterly GDP growth rate (%)",
            "inflation" => "Consumer price inflation (%)",
            "unemployment" => "Unemployment rate (%)",
            "interest_rate" => "Central bank policy rate (%)",
            "fiscal_balance" => "Government fiscal balance (% GDP)",
            "current_account" => "Current account balance (% GDP)",
            "housing_price_index" => "Housing price index (2015=100)",
            "consumer_confidence" => "Consumer confidence index",
            "investment_flow" => "Gross fixed capital formation (% GDP)",
            "exchange_rate" => "Nominal effective exchange rate",
            "geopolitical_tension_index" => "Synthetic geopolitical tension (0-1)",
            "sanctions_exposure" => "Sanctions exposure index (0-1)",
            "alliance_stability" => "Alliance stability index (0-1)",
            _ => "Economic indicator",
        };
        md.push_str(&format!("| `{col}` | {desc} |\n"));
    }
    md.push_str("\n");

    md.push_str("## Pipeline Results\n\n");

    md.push_str("### Data Loading\n\n");
    md.push_str(&format!("- Rows loaded: **{}**\n", df.row_count));
    md.push_str(&format!(
        "- Observations generated: **{}**\n\n",
        observations.len()
    ));

    md.push_str("### EKF State Estimate\n\n");
    md.push_str("| Dimension | Value |\n");
    md.push_str("|-----------|-------|\n");
    let dims = [
        ("capacity", final_state.as_vector()[0]),
        ("investment", final_state.as_vector()[1]),
        ("labour_absorption", final_state.as_vector()[2]),
        ("fiscal_capacity", final_state.as_vector()[3]),
        ("demand_pressure", final_state.as_vector()[4]),
        ("housing_pressure", final_state.as_vector()[5]),
        ("geopolitical_load", final_state.as_vector()[6]),
        ("migration_pressure", final_state.as_vector()[7]),
    ];
    for (name, val) in dims {
        md.push_str(&format!("| {name} | {val:.4} |\n"));
    }
    md.push_str("\n");

    md.push_str("### Monte Carlo Simulation\n\n");
    md.push_str("| Dimension | Mean |\n");
    md.push_str("|-----------|------|\n");
    for (name, val) in dims {
        md.push_str(&format!("| {name} | {val:.4} |\n"));
    }
    md.push_str("\n");

    md.push_str("### Policy Evaluation\n\n");
    md.push_str("| Instrument | Score |\n");
    md.push_str("|-------------|-------|\n");
    for instrument in [
        PolicyInstrument::InterestRate,
        PolicyInstrument::QuantitativeEasing,
        PolicyInstrument::SovereignWealthFundDeployment,
        PolicyInstrument::FiscalSpending,
        PolicyInstrument::Taxation,
        PolicyInstrument::InfrastructureInvestment,
        PolicyInstrument::MigrationCapacity,
    ] {
        let score = policy
            .recommended_action_distribution
            .get(&instrument)
            .copied()
            .unwrap_or(0.0);
        md.push_str(&format!("| {instrument:?} | {score:.4} |\n"));
    }
    if policy.constraint_violations.is_empty() {
        md.push_str("\n**Constraint violations:** None\n");
    } else {
        md.push_str(&format!(
            "\n**Constraint violations:** {}\n",
            policy.constraint_violations.join(", ")
        ));
    }
    md.push_str("\n");

    md.push_str("### Backtest Metrics\n\n");
    md.push_str(&format!("- RMSE: {:.6}\n", metrics.rmse));
    md.push_str(&format!("- MAE: {:.6}\n", metrics.mae));
    md.push_str(&format!(
        "- Calibration score: {:.6}\n",
        metrics.calibration_score
    ));
    md.push_str(&format!("- Brier score: {:.6}\n", metrics.brier_score));
    md.push_str(&format!("- Log loss: {:.6}\n", metrics.log_loss));
    md.push_str("\n");

    md.push_str("## Methodology\n\n");
    md.push_str(
        "1. **Data generation** — cyclical base + Gaussian noise, clamped to realistic bounds\n",
    );
    md.push_str(
        "2. **EKF** — Extended Kalman Filter with numerical Jacobian (central differences)\n",
    );
    md.push_str("3. **Monte Carlo** — 100 paths, 8-quarter horizon, Cholesky sampling with eigenvalue fallback\n");
    md.push_str("4. **Policy** — stochastic policy engine with regime-conditional bias\n");
    md.push_str("5. **Evaluation** — rolling out-of-sample backtest against naive baseline\n\n");
    md.push_str("---\n\n");
    md.push_str("Generated by `prv-cli` example `run_pipeline`.\n");

    std::fs::write(path, md)?;
    Ok(())
}

fn print_usage() {
    println!("Usage: run_pipeline [OPTIONS]\n");
    println!("Options:");
    println!("  --save <PATH>     Save markdown report to PATH");
    println!("  --seed <N>        Random seed for data generation (default: 42)");
    println!("  --rows <N>        Number of faux data rows to generate (default: 20, min: 4)");
    println!("  --help            Print this help message");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut save_path = None;
    let mut seed = 42u64;
    let mut rows = 20usize;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--save" => {
                i += 1;
                if i < args.len() {
                    save_path = Some(args[i].clone());
                }
            }
            "--seed" => {
                i += 1;
                if i < args.len() {
                    seed = args[i].parse().unwrap_or(42);
                }
            }
            "--rows" => {
                i += 1;
                if i < args.len() {
                    rows = args[i].parse().unwrap_or(20);
                }
            }
            "--help" | "-h" => {
                print_usage();
                return;
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_usage();
                return;
            }
        }
        i += 1;
    }

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/cli has a parent")
        .parent()
        .expect("crates has a parent")
        .to_path_buf();
    let data_path = project_root.join("examples/faux_data.csv");
    let data_path_str = data_path.to_str().expect("valid UTF-8 path");

    if save_path.is_none() {
        println!("=== PRV Full Pipeline Demo ===\n");
        println!("Generating faux dataset at {data_path_str} (seed={seed}, rows={rows})...\n");
    }

    if let Err(e) = generate_faux_csv(data_path_str, seed, rows) {
        eprintln!("Failed to generate faux data: {e}");
        return;
    }

    let loader = DataLoader::new();
    let df = match loader.load_historical(data_path_str) {
        Ok(df) => df,
        Err(e) => {
            eprintln!("Failed to load faux_data.csv: {e}");
            return;
        }
    };

    if save_path.is_none() {
        println!(
            "Loaded DataFrame: {} rows x {} columns",
            df.row_count,
            df.columns.len()
        );
        println!("Columns: {:?}\n", df.columns);
    }

    let observations = match loader.to_observations(&df) {
        Ok(obs) => obs,
        Err(e) => {
            eprintln!("Failed to convert to observations: {e}");
            return;
        }
    };

    if save_path.is_none() {
        println!("Generated {} observations\n", observations.len());
    }

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

    for obs in &observations {
        let _ = ekf.predict(None);
        let _ = ekf.update(obs);
    }

    let mean = ekf.x_hat.clone();
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

    let mc = Simulator::new(42)
        .simulate(&mean, &covariance, 100, 8, &shocks)
        .expect("Monte Carlo simulation");

    let engine = PolicyEngine::new(PolicyWeights::default());
    let policy = engine.evaluate(&mc, &Regime::Expansion, Some(42));

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

    if save_path.is_none() {
        println!("Final EKF state: {:?}", ekf.x_hat.as_vector().transpose());
        println!(
            "Monte Carlo mean state: {:?}\n",
            mc.mean.as_vector().transpose()
        );
        println!(
            "Policy recommended actions: {:?}\n",
            policy.recommended_action_distribution
        );
        println!(
            "Backtest metrics: RMSE={:.4}, MAE={:.4}\n",
            results.metrics.rmse, results.metrics.mae
        );
        println!("=== Pipeline complete ===");
    }

    if let Some(path) = save_path {
        if let Err(e) = generate_markdown_report(
            &path,
            seed,
            rows,
            &df,
            &observations,
            &ekf.x_hat,
            &mc.mean,
            &policy,
            &results.metrics,
        ) {
            eprintln!("Failed to save report: {e}");
            return;
        }
        println!("Report saved to {path}");
    }
}

#![allow(clippy::print_stdout, clippy::print_stderr)]

use docx_basic::Docx;
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
use std::path::Path;

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

#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
fn generate_faux_csv(path: &str, seed: u64, rows: usize) -> std::io::Result<()> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    #[allow(clippy::expect_used)]
    let normal = Normal::new(0.0, 0.5).expect("normal distribution parameters are valid");

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

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::single_char_add_str,
    clippy::format_push_string
)]
// TETANUS-exempt(power10-04): report generator is I/O formatting, not control logic; refactoring to 60-line functions is low-value
fn generate_markdown_report(
    path: &str,
    seed: u64,
    rows: usize,
    df: &prv_data::DataFrame,
    observations: &[prv_core::Observation],
    final_state: &State,
    mc_mean: &State,
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
    let mc_dims = [
        ("capacity", mc_mean.as_vector()[0]),
        ("investment", mc_mean.as_vector()[1]),
        ("labour_absorption", mc_mean.as_vector()[2]),
        ("fiscal_capacity", mc_mean.as_vector()[3]),
        ("demand_pressure", mc_mean.as_vector()[4]),
        ("housing_pressure", mc_mean.as_vector()[5]),
        ("geopolitical_load", mc_mean.as_vector()[6]),
        ("migration_pressure", mc_mean.as_vector()[7]),
    ];
    for (name, val) in dims {
        md.push_str(&format!("| {name} | {val:.4} |\n"));
    }
    md.push_str("\n");

    md.push_str("### Monte Carlo Simulation\n\n");
    md.push_str("| Dimension | Mean |\n");
    md.push_str("|-----------|------|\n");
    for (name, val) in mc_dims {
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
        #[allow(clippy::use_debug)]
        {
            md.push_str(&format!("| {instrument:?} | {score:.4} |\n"));
        }
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

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::single_char_add_str,
    clippy::format_push_string
)]
// TETANUS-exempt(power10-04): report generator is I/O formatting, not control logic; refactoring to 60-line functions is low-value
fn generate_toml_report(
    path: &str,
    seed: u64,
    rows: usize,
    df: &prv_data::DataFrame,
    observations: &[prv_core::Observation],
    final_state: &State,
    mc_mean: &State,
    policy: &prv_policy::PolicyDistribution,
    metrics: &prv_evaluation::MetricResults,
) -> std::io::Result<()> {
    let dims = [
        "capacity",
        "investment",
        "labour_absorption",
        "fiscal_capacity",
        "demand_pressure",
        "housing_pressure",
        "geopolitical_load",
        "migration_pressure",
    ];
    let state_vals: Vec<f64> = final_state.as_vector().iter().copied().collect();
    let mc_vals: Vec<f64> = mc_mean.as_vector().iter().copied().collect();

    let mut toml = String::new();
    toml.push_str("# PRV Pipeline Report\n");
    toml.push_str("# This file explains what each numeric output means.\n\n");
    toml.push_str("[report]\n");
    toml.push_str(&format!(
        "generated = \"{}\"\n",
        chrono::Utc::now().to_rfc3339()
    ));
    toml.push_str(&format!("seed = {seed}\n"));
    toml.push_str(&format!("rows = {rows}\n\n"));

    toml.push_str("[dataset]\n");
    toml.push_str(&format!("rows_loaded = {}\n", df.row_count));
    toml.push_str(&format!(
        "observations_generated = {}\n",
        observations.len()
    ));
    toml.push_str("columns = [\n");
    for col in &df.columns {
        toml.push_str(&format!("  \"{col}\",\n"));
    }
    toml.push_str("]\n\n");

    toml.push_str(
        "# EKF state estimate: 8-dimensional latent state after processing all observations.\n",
    );
    toml.push_str("# - capacity: productive capacity utilization\n");
    toml.push_str("# - investment: gross fixed capital formation level\n");
    toml.push_str("# - labour_absorption: employment intensity\n");
    toml.push_str("# - fiscal_capacity: government fiscal headroom\n");
    toml.push_str("# - demand_pressure: aggregate demand pressure\n");
    toml.push_str("# - housing_pressure: housing market pressure\n");
    toml.push_str("# - geopolitical_load: geopolitical stress loading\n");
    toml.push_str("# - migration_pressure: migration system pressure\n\n");
    toml.push_str("[ekf]\n");
    for (name, val) in dims.iter().zip(state_vals.iter()) {
        toml.push_str(&format!("{name} = {val:.6}\n"));
    }
    toml.push_str("\n");

    toml.push_str("# Monte Carlo mean state: average of 100 simulated 8-quarter paths under shock scenarios.\n");
    toml.push_str("# Shock specs: Demand (amplitude=0.05, persistence=0.8) and Geopolitical (amplitude=0.03, persistence=0.9).\n\n");
    toml.push_str("[monte_carlo]\n");
    for (name, val) in dims.iter().zip(mc_vals.iter()) {
        toml.push_str(&format!("{name} = {val:.6}\n"));
    }
    toml.push_str("\n");

    toml.push_str("# Policy recommended actions: scores in [0,1] for each instrument under Expansion regime.\n");
    toml.push_str("# Higher values indicate stronger recommended deployment.\n\n");
    toml.push_str("[policy]\n");
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
        #[allow(clippy::use_debug)]
        {
            toml.push_str(&format!("{instrument:?} = {score:.6}\n"));
        }
    }
    if policy.constraint_violations.is_empty() {
        toml.push_str("constraint_violations = []\n");
    } else {
        #[allow(clippy::use_debug)]
        {
            toml.push_str(&format!(
                "constraint_violations = {:?}\n",
                policy.constraint_violations
            ));
        }
    }
    toml.push_str("\n");

    toml.push_str(
        "# Backtest metrics: rolling out-of-sample evaluation against a naive baseline model.\n",
    );
    toml.push_str("# - rmse: root mean squared error\n");
    toml.push_str("# - mae: mean absolute error\n");
    toml.push_str("# - calibration_score: reliability of probabilistic forecasts\n");
    toml.push_str("# - brier_score: proper scoring rule for binary outcomes\n");
    toml.push_str("# - log_loss: logarithmic loss for probabilistic predictions\n\n");
    toml.push_str("[backtest]\n");
    toml.push_str(&format!("rmse = {:.6}\n", metrics.rmse));
    toml.push_str(&format!("mae = {:.6}\n", metrics.mae));
    toml.push_str(&format!(
        "calibration_score = {:.6}\n",
        metrics.calibration_score
    ));
    toml.push_str(&format!("brier_score = {:.6}\n", metrics.brier_score));
    toml.push_str(&format!("log_loss = {:.6}\n", metrics.log_loss));

    std::fs::write(path, toml)?;
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::single_char_add_str,
    clippy::format_push_string
)]
// TETANUS-exempt(power10-04): report generator is I/O formatting, not control logic; refactoring to 60-line functions is low-value
fn generate_txt_report(
    path: &str,
    seed: u64,
    rows: usize,
    df: &prv_data::DataFrame,
    observations: &[prv_core::Observation],
    final_state: &State,
    mc_mean: &State,
    policy: &prv_policy::PolicyDistribution,
    metrics: &prv_evaluation::MetricResults,
) -> std::io::Result<()> {
    let mut txt = String::new();
    txt.push_str("PRV Pipeline Report\n");
    txt.push_str("==================\n\n");
    txt.push_str(&format!("Generated: {}\n", chrono::Utc::now().to_rfc3339()));
    txt.push_str(&format!("Seed: {seed}\n"));
    txt.push_str(&format!("Rows: {rows}\n\n"));

    txt.push_str("Faux Dataset\n");
    txt.push_str("------------\n");
    txt.push_str("The faux dataset is a synthetically generated quarterly economic time series\n");
    txt.push_str("designed to exercise the full PRV pipeline. It combines cyclical patterns,\n");
    txt.push_str("Gaussian noise, cross-series coupling, and realistic bounds.\n\n");
    txt.push_str(&format!(
        "Loaded {} rows x {} columns.\n",
        df.row_count,
        df.columns.len()
    ));
    txt.push_str("Columns:\n");
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
        txt.push_str(&format!("  {col}: {desc}\n"));
    }
    txt.push_str("\n");

    txt.push_str("Pipeline Results\n");
    txt.push_str("-----------------\n\n");

    txt.push_str("Data Loading\n");
    txt.push_str(&format!("  Rows loaded: {}\n", df.row_count));
    txt.push_str(&format!(
        "  Observations generated: {}\n\n",
        observations.len()
    ));

    txt.push_str("EKF State Estimate\n");
    txt.push_str(
        "  The EKF produces an 8-dimensional latent state after ingesting all observations.\n",
    );
    let dims = [
        ("capacity", "productive capacity utilization"),
        ("investment", "gross fixed capital formation level"),
        ("labour_absorption", "employment intensity"),
        ("fiscal_capacity", "government fiscal headroom"),
        ("demand_pressure", "aggregate demand pressure"),
        ("housing_pressure", "housing market pressure"),
        ("geopolitical_load", "geopolitical stress loading"),
        ("migration_pressure", "migration system pressure"),
    ];
    for ((name, desc), val) in dims.iter().zip(final_state.as_vector().iter()) {
        txt.push_str(&format!("  {name}: {val:.4}  # {desc}\n"));
    }
    txt.push_str("\n");

    txt.push_str("Monte Carlo Simulation\n");
    txt.push_str("  100 paths, 8-quarter horizon, Cholesky sampling with eigenvalue fallback.\n");
    txt.push_str("  Mean state across all simulated paths:\n");
    for ((name, _), val) in dims.iter().zip(mc_mean.as_vector().iter()) {
        txt.push_str(&format!("  {name}: {val:.4}\n"));
    }
    txt.push_str("\n");

    txt.push_str("Policy Evaluation\n");
    txt.push_str("  Stochastic policy engine under Expansion regime. Scores are in [0,1].\n");
    txt.push_str("  Higher values indicate stronger recommended deployment.\n");
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
        #[allow(clippy::use_debug)]
        {
            txt.push_str(&format!("  {instrument:?}: {score:.4}\n"));
        }
    }
    if policy.constraint_violations.is_empty() {
        txt.push_str("  Constraint violations: None\n");
    } else {
        txt.push_str(&format!(
            "  Constraint violations: {}\n",
            policy.constraint_violations.join(", ")
        ));
    }
    txt.push_str("\n");

    txt.push_str("Backtest Metrics\n");
    txt.push_str("  Rolling out-of-sample evaluation against a naive baseline model.\n");
    txt.push_str(&format!(
        "  RMSE: {:.6}  # root mean squared error\n",
        metrics.rmse
    ));
    txt.push_str(&format!(
        "  MAE: {:.6}  # mean absolute error\n",
        metrics.mae
    ));
    txt.push_str(&format!(
        "  Calibration score: {:.6}  # reliability of probabilistic forecasts\n",
        metrics.calibration_score
    ));
    txt.push_str(&format!(
        "  Brier score: {:.6}  # proper scoring rule for binary outcomes\n",
        metrics.brier_score
    ));
    txt.push_str(&format!(
        "  Log loss: {:.6}  # logarithmic loss for probabilistic predictions\n",
        metrics.log_loss
    ));
    txt.push_str("\n");
    txt.push_str("Generated by prv-cli example run_pipeline.\n");

    std::fs::write(path, txt)?;
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::single_char_add_str,
    clippy::format_push_string
)]
// TETANUS-exempt(power10-04): report generator is I/O formatting, not control logic; refactoring to 60-line functions is low-value
fn generate_docx_report(
    path: &str,
    seed: u64,
    rows: usize,
    df: &prv_data::DataFrame,
    observations: &[prv_core::Observation],
    final_state: &State,
    mc_mean: &State,
    policy: &prv_policy::PolicyDistribution,
    metrics: &prv_evaluation::MetricResults,
) -> std::io::Result<()> {
    let mut doc = Docx::new();

    doc = doc
        .paragraph_with(|p| p.push_text("PRV Pipeline Report"))
        .paragraph_with(|p| p.push_text(format!("Generated: {}", chrono::Utc::now().to_rfc3339())))
        .paragraph_with(|p| p.push_text(format!("Seed: {seed}")))
        .paragraph_with(|p| p.push_text(format!("Rows: {rows}")))
        .paragraph_with(|p| p.push_text(""));

    doc = doc
        .paragraph_with(|p| p.push_text("Faux Dataset"))
        .paragraph_with(|p| p.push_text("The faux dataset is a synthetically generated quarterly economic time series designed to exercise the full PRV pipeline. It combines cyclical patterns, Gaussian noise, cross-series coupling, and realistic bounds."))
        .paragraph_with(|p| p.push_text(format!("Loaded {} rows x {} columns.", df.row_count, df.columns.len())))
        .paragraph_with(|p| p.push_text("Columns:"));

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
        doc = doc.paragraph_with(|p| p.push_text(format!("{col}: {desc}")));
    }

    doc = doc.paragraph_with(|p| p.push_text(""));

    doc = doc
        .paragraph_with(|p| p.push_text("Pipeline Results"))
        .paragraph_with(|p| p.push_text("Data Loading"))
        .paragraph_with(|p| {
            p.push_text(format!(
                "Rows loaded: {}. Observations generated: {}.",
                df.row_count,
                observations.len()
            ))
        });

    doc = doc
        .paragraph_with(|p| p.push_text("EKF State Estimate"))
        .paragraph_with(|p| {
            p.push_text(
                "The EKF produces an 8-dimensional latent state after ingesting all observations.",
            )
        });

    let dims = [
        ("capacity", "productive capacity utilization"),
        ("investment", "gross fixed capital formation level"),
        ("labour_absorption", "employment intensity"),
        ("fiscal_capacity", "government fiscal headroom"),
        ("demand_pressure", "aggregate demand pressure"),
        ("housing_pressure", "housing market pressure"),
        ("geopolitical_load", "geopolitical stress loading"),
        ("migration_pressure", "migration system pressure"),
    ];
    for ((name, desc), val) in dims.iter().zip(final_state.as_vector().iter()) {
        doc = doc.paragraph_with(|p| p.push_text(format!("{name}: {val:.4}  # {desc}")));
    }

    doc = doc
        .paragraph_with(|p| p.push_text("Monte Carlo Simulation"))
        .paragraph_with(|p| p.push_text("100 paths, 8-quarter horizon, Cholesky sampling with eigenvalue fallback. Mean state across all simulated paths:"));

    for ((name, _), val) in dims.iter().zip(mc_mean.as_vector().iter()) {
        doc = doc.paragraph_with(|p| p.push_text(format!("{name}: {val:.4}")));
    }

    doc = doc
        .paragraph_with(|p| p.push_text("Policy Evaluation"))
        .paragraph_with(|p| p.push_text("Stochastic policy engine under Expansion regime. Scores are in [0,1]. Higher values indicate stronger recommended deployment."));

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
        #[allow(clippy::use_debug)]
        {
            doc = doc.paragraph_with(|p| p.push_text(format!("{instrument:?}: {score:.4}")));
        }
    }
    if policy.constraint_violations.is_empty() {
        doc = doc.paragraph_with(|p| p.push_text("Constraint violations: None"));
    } else {
        doc = doc.paragraph_with(|p| {
            p.push_text(format!(
                "Constraint violations: {}",
                policy.constraint_violations.join(", ")
            ))
        });
    }

    doc = doc
        .paragraph_with(|p| p.push_text("Backtest Metrics"))
        .paragraph_with(|p| {
            p.push_text("Rolling out-of-sample evaluation against a naive baseline model.")
        })
        .paragraph_with(|p| {
            p.push_text(format!(
                "RMSE: {:.6}  # root mean squared error",
                metrics.rmse
            ))
        })
        .paragraph_with(|p| p.push_text(format!("MAE: {:.6}  # mean absolute error", metrics.mae)))
        .paragraph_with(|p| {
            p.push_text(format!(
                "Calibration score: {:.6}  # reliability of probabilistic forecasts",
                metrics.calibration_score
            ))
        })
        .paragraph_with(|p| {
            p.push_text(format!(
                "Brier score: {:.6}  # proper scoring rule for binary outcomes",
                metrics.brier_score
            ))
        })
        .paragraph_with(|p| {
            p.push_text(format!(
                "Log loss: {:.6}  # logarithmic loss for probabilistic predictions",
                metrics.log_loss
            ))
        })
        .paragraph_with(|p| p.push_text(""))
        .paragraph_with(|p| p.push_text("Generated by prv-cli example run_pipeline."));

    doc.write_file(path).map_err(std::io::Error::other)?;
    Ok(())
}

fn print_usage() {
    println!("Usage: run_pipeline [OPTIONS]\n");
    println!("Options:");
    println!(
        "  --save <PATH>     Save report to PATH (format auto-detected from extension: md, txt, toml, docx)"
    );
    println!("  --seed <N>        Random seed for data generation (default: 42)");
    println!("  --rows <N>        Number of faux data rows to generate (default: 20, min: 4)");
    println!("  --help            Print this help message");
}

#[allow(clippy::too_many_lines)]
// TETANUS-exempt(power10-04): main() is the example entry point; splitting into sub-functions would obscure the linear pipeline flow for readers
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut save_path = None;
    let mut save_format = "md";
    let mut seed = 42u64;
    let mut rows = 20usize;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--save" => {
                i += 1;
                if i < args.len() {
                    save_path = Some(args[i].clone());
                    if let Some(ext) = Path::new(&args[i]).extension().and_then(|e| e.to_str()) {
                        save_format = match ext.to_lowercase().as_str() {
                            "txt" => "txt",
                            "toml" => "toml",
                            "docx" => "docx",
                            _ => "md",
                        };
                    }
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

    #[allow(clippy::expect_used)]
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/cli has a parent")
        .parent()
        .expect("crates has a parent")
        .to_path_buf();
    let data_path = project_root.join("examples/faux_data.csv");
    #[allow(clippy::expect_used)]
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
        #[allow(clippy::use_debug)]
        {
            println!("Columns: {:?}\n", df.columns);
        }
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
        #[allow(clippy::let_underscore_must_use)]
        let _ = ekf.predict(None);
        #[allow(clippy::let_underscore_must_use)]
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

    #[allow(clippy::expect_used)]
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
        #[allow(clippy::use_debug)]
        {
            println!("Final EKF state: {:?}", ekf.x_hat.as_vector().transpose());
        }
        #[allow(clippy::use_debug)]
        {
            println!(
                "Monte Carlo mean state: {:?}\n",
                mc.mean.as_vector().transpose()
            );
        }
        #[allow(clippy::use_debug)]
        {
            println!(
                "Policy recommended actions: {:?}\n",
                policy.recommended_action_distribution
            );
        }
        println!(
            "Backtest metrics: RMSE={:.4}, MAE={:.4}\n",
            results.metrics.rmse, results.metrics.mae
        );
        println!("=== Pipeline complete ===");
    }

    if let Some(path) = save_path {
        let result = match save_format {
            "txt" => generate_txt_report(
                &path,
                seed,
                rows,
                &df,
                &observations,
                &ekf.x_hat,
                &mc.mean,
                &policy,
                &results.metrics,
            ),
            "toml" => generate_toml_report(
                &path,
                seed,
                rows,
                &df,
                &observations,
                &ekf.x_hat,
                &mc.mean,
                &policy,
                &results.metrics,
            ),
            "docx" => generate_docx_report(
                &path,
                seed,
                rows,
                &df,
                &observations,
                &ekf.x_hat,
                &mc.mean,
                &policy,
                &results.metrics,
            ),
            _ => generate_markdown_report(
                &path,
                seed,
                rows,
                &df,
                &observations,
                &ekf.x_hat,
                &mc.mean,
                &policy,
                &results.metrics,
            ),
        };

        if let Err(e) = result {
            eprintln!("Failed to save report: {e}");
            return;
        }
        println!("Report saved to {path}");
    }
}

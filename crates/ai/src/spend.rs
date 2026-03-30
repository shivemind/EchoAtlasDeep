#![allow(dead_code, unused_imports, unused_variables)]
//! Model spend tracking, pricing, budget controls, and analytics.
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Per-model pricing (USD per million tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub provider: String,
    pub model_id: String,
    pub input_per_mtok: f64,   // $ per 1M input tokens
    pub output_per_mtok: f64,  // $ per 1M output tokens
    pub context_window: u32,
    pub supports_vision: bool,
    pub supports_functions: bool,
    pub latency_tier: LatencyTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LatencyTier { Fast, Medium, Slow }

impl ModelPricing {
    pub fn cost_usd(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        (input_tokens as f64 / 1_000_000.0) * self.input_per_mtok
            + (output_tokens as f64 / 1_000_000.0) * self.output_per_mtok
    }
}

/// Embedded pricing table (updated via bundled data).
pub fn pricing_table() -> Vec<ModelPricing> {
    vec![
        // Anthropic Claude
        ModelPricing { provider: "claude".into(), model_id: "claude-opus-4-6".into(),    input_per_mtok: 15.0,  output_per_mtok: 75.0,  context_window: 200_000, supports_vision: true,  supports_functions: true,  latency_tier: LatencyTier::Slow },
        ModelPricing { provider: "claude".into(), model_id: "claude-sonnet-4-6".into(),  input_per_mtok: 3.0,   output_per_mtok: 15.0,  context_window: 200_000, supports_vision: true,  supports_functions: true,  latency_tier: LatencyTier::Medium },
        ModelPricing { provider: "claude".into(), model_id: "claude-haiku-4-5".into(),   input_per_mtok: 0.8,   output_per_mtok: 4.0,   context_window: 200_000, supports_vision: true,  supports_functions: true,  latency_tier: LatencyTier::Fast },
        // OpenAI
        ModelPricing { provider: "openai".into(),  model_id: "gpt-4o".into(),            input_per_mtok: 2.5,   output_per_mtok: 10.0,  context_window: 128_000, supports_vision: true,  supports_functions: true,  latency_tier: LatencyTier::Medium },
        ModelPricing { provider: "openai".into(),  model_id: "gpt-4o-mini".into(),       input_per_mtok: 0.15,  output_per_mtok: 0.6,   context_window: 128_000, supports_vision: true,  supports_functions: true,  latency_tier: LatencyTier::Fast },
        ModelPricing { provider: "openai".into(),  model_id: "o1".into(),                input_per_mtok: 15.0,  output_per_mtok: 60.0,  context_window: 128_000, supports_vision: false, supports_functions: false, latency_tier: LatencyTier::Slow },
        ModelPricing { provider: "openai".into(),  model_id: "o3-mini".into(),           input_per_mtok: 1.1,   output_per_mtok: 4.4,   context_window: 128_000, supports_vision: false, supports_functions: true,  latency_tier: LatencyTier::Medium },
        // Google Gemini
        ModelPricing { provider: "gemini".into(),  model_id: "gemini-2.0-flash".into(),  input_per_mtok: 0.075, output_per_mtok: 0.30,  context_window: 1_000_000, supports_vision: true, supports_functions: true, latency_tier: LatencyTier::Fast },
        ModelPricing { provider: "gemini".into(),  model_id: "gemini-2.0-pro".into(),    input_per_mtok: 1.25,  output_per_mtok: 5.0,   context_window: 2_000_000, supports_vision: true, supports_functions: true, latency_tier: LatencyTier::Medium },
        ModelPricing { provider: "gemini".into(),  model_id: "gemini-1.5-flash".into(),  input_per_mtok: 0.075, output_per_mtok: 0.30,  context_window: 1_000_000, supports_vision: true, supports_functions: true, latency_tier: LatencyTier::Fast },
        // Ollama (free / local)
        ModelPricing { provider: "ollama".into(),  model_id: "llama3".into(),            input_per_mtok: 0.0,   output_per_mtok: 0.0,   context_window: 8_192,   supports_vision: false, supports_functions: false, latency_tier: LatencyTier::Medium },
        ModelPricing { provider: "ollama".into(),  model_id: "mistral".into(),           input_per_mtok: 0.0,   output_per_mtok: 0.0,   context_window: 32_768,  supports_vision: false, supports_functions: false, latency_tier: LatencyTier::Medium },
        ModelPricing { provider: "ollama".into(),  model_id: "deepseek-coder".into(),    input_per_mtok: 0.0,   output_per_mtok: 0.0,   context_window: 16_384,  supports_vision: false, supports_functions: false, latency_tier: LatencyTier::Medium },
    ]
}

/// One usage record (one completed AI request).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub timestamp: u64,
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub session_id: String,
}

/// Aggregated stats per model.
#[derive(Debug, Clone, Default)]
pub struct ModelStats {
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

pub struct SpendTracker {
    pricing: Vec<ModelPricing>,
    records: RwLock<Vec<UsageRecord>>,
    session_id: String,
    /// Budget in USD (0 = unlimited).
    pub session_budget_usd: RwLock<f64>,
    pub daily_budget_usd: RwLock<f64>,
    session_cost: RwLock<f64>,
}

impl SpendTracker {
    pub fn new() -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        Self {
            pricing: pricing_table(),
            records: RwLock::new(Vec::new()),
            session_id,
            session_budget_usd: RwLock::new(0.0),
            daily_budget_usd: RwLock::new(0.0),
            session_cost: RwLock::new(0.0),
        }
    }

    /// Find pricing for (provider, model).
    pub fn lookup_pricing(&self, provider: &str, model: &str) -> Option<&ModelPricing> {
        self.pricing.iter().find(|p| {
            p.provider == provider && (p.model_id == model || model.contains(&p.model_id))
        })
    }

    /// Record a completed AI request. Returns the cost of this request.
    pub fn record(&self, provider: &str, model: &str, input: u64, output: u64) -> f64 {
        let cost = self.lookup_pricing(provider, model)
            .map(|p| p.cost_usd(input, output))
            .unwrap_or(0.0);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.records.write().push(UsageRecord {
            timestamp: now,
            provider: provider.to_string(),
            model: model.to_string(),
            input_tokens: input,
            output_tokens: output,
            cost_usd: cost,
            session_id: self.session_id.clone(),
        });

        *self.session_cost.write() += cost;
        cost
    }

    /// Total session cost so far.
    pub fn session_cost(&self) -> f64 {
        *self.session_cost.read()
    }

    /// Per-model aggregated stats.
    pub fn stats_by_model(&self) -> Vec<(String, String, ModelStats)> {
        let mut map: HashMap<(String, String), ModelStats> = HashMap::new();
        for rec in self.records.read().iter() {
            let entry = map.entry((rec.provider.clone(), rec.model.clone())).or_default();
            entry.requests += 1;
            entry.input_tokens += rec.input_tokens;
            entry.output_tokens += rec.output_tokens;
            entry.cost_usd += rec.cost_usd;
        }
        let mut result: Vec<_> = map.into_iter()
            .map(|((p, m), s)| (p, m, s))
            .collect();
        result.sort_by(|a, b| b.2.cost_usd.partial_cmp(&a.2.cost_usd).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// Daily usage totals (last 30 days).
    pub fn daily_totals(&self) -> Vec<(String, f64)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut days: HashMap<String, f64> = HashMap::new();
        for rec in self.records.read().iter() {
            if now - rec.timestamp > 30 * 86400 { continue; }
            let day = format_day(rec.timestamp);
            *days.entry(day).or_insert(0.0) += rec.cost_usd;
        }
        let mut v: Vec<_> = days.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    /// Budget check: returns fraction of session budget used (0.0–1.0+). None if no budget set.
    pub fn session_budget_fraction(&self) -> Option<f64> {
        let budget = *self.session_budget_usd.read();
        if budget <= 0.0 { return None; }
        Some(self.session_cost() / budget)
    }

    /// True if session budget is exceeded.
    pub fn session_over_budget(&self) -> bool {
        self.session_budget_fraction().map(|f| f >= 1.0).unwrap_or(false)
    }

    /// True if we're approaching budget (>= 80%).
    pub fn session_budget_warning(&self) -> bool {
        self.session_budget_fraction().map(|f| f >= 0.8).unwrap_or(false)
    }

    /// Status string for status bar.
    pub fn status_string(&self) -> String {
        let cost = self.session_cost();
        if cost < 0.001 { return String::new(); }
        format!("${:.4}", cost)
    }

    /// Full breakdown text.
    pub fn breakdown_text(&self) -> String {
        let stats = self.stats_by_model();
        if stats.is_empty() { return "No usage recorded this session.".to_string(); }
        let mut lines = vec![
            format!("{:<20} {:>8} {:>10} {:>10} {:>10}",
                "Model", "Reqs", "Input Tok", "Output Tok", "Cost USD"),
            "-".repeat(62),
        ];
        for (provider, model, s) in &stats {
            lines.push(format!("{:<20} {:>8} {:>10} {:>10} {:>10.4}",
                format!("{provider}/{model}"), s.requests, s.input_tokens, s.output_tokens, s.cost_usd));
        }
        lines.push("-".repeat(62));
        lines.push(format!("Session total: ${:.4}", self.session_cost()));
        lines.join("\n")
    }
}

fn format_day(ts: u64) -> String {
    // Simple YYYY-MM-DD without chrono dependency
    let secs = ts;
    let days_since_epoch = secs / 86400;
    // Approximate date string (good enough for display)
    format!("day-{days_since_epoch}")
}

impl Default for SpendTracker {
    fn default() -> Self { Self::new() }
}

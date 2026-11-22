//! Presentation state for the Data surface. It reads the local store through
//! [`AppState`]; no generated Slint type and no raw hex leaves this crate.

use slint::{ModelRc, SharedString, VecModel};

use crate::bridge::AppError;
use crate::{AppState, AppWindow};

/// Local display preference. Not sent to the watch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    Metric,
    Imperial,
}

impl Units {
    pub fn from_label(label: &str) -> Self {
        if label == "imperial" {
            Self::Imperial
        } else {
            Self::Metric
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Metric => "metric",
            Self::Imperial => "imperial",
        }
    }

    fn toggle(self) -> Self {
        match self {
            Self::Metric => Self::Imperial,
            Self::Imperial => Self::Metric,
        }
    }
}

#[derive(Debug, Clone)]
struct Series {
    key: String,
    values: Vec<(i64, String)>,
}

/// State behind the Data surface: history chart, export preview, delete
/// confirmation and the units toggle.
pub struct DataView {
    series: Vec<Series>,
    selected: usize,
    points: Vec<f64>,
    labels: Vec<String>,
    summary: String,
    preview: String,
    status: String,
    pending_delete: bool,
    units: Units,
}

impl Default for DataView {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            selected: 0,
            points: Vec::new(),
            labels: Vec::new(),
            summary: "No numeric history yet".into(),
            preview: String::new(),
            status: "Local store · nothing leaves this device".into(),
            pending_delete: false,
            units: Units::Metric,
        }
    }
}

impl DataView {
    /// Reload numeric history series from the store, keeping the selection.
    pub fn reload(&mut self, state: &AppState) -> Result<(), AppError> {
        let keep = self
            .series
            .get(self.selected)
            .map(|series| series.key.clone());
        let mut series = Vec::new();
        for key in state.reading_keys()? {
            if key == "units" {
                continue; // a preference, not a metric
            }
            let values: Vec<(i64, String)> = state
                .readings(&key)?
                .into_iter()
                .filter(|(_, value)| value.parse::<f64>().is_ok())
                .collect();
            if values.len() >= 2 {
                series.push(Series { key, values });
            }
        }
        self.selected = keep
            .as_deref()
            .and_then(|key| series.iter().position(|item| item.key == key))
            .unwrap_or(0);
        self.series = series;
        self.recompute();
        Ok(())
    }

    /// Load the persisted units preference, if the store has one.
    pub fn load_units(&mut self, state: &AppState) {
        if let Some(units) = state.units() {
            self.units = Units::from_label(&units);
        }
    }

    fn recompute(&mut self) {
        match self.series.get(self.selected) {
            Some(current) => {
                let peak = current
                    .values
                    .iter()
                    .filter_map(|(_, value)| value.parse::<f64>().ok())
                    .fold(0.0_f64, f64::max);
                self.points = current
                    .values
                    .iter()
                    .map(|(_, value)| {
                        let number = value.parse::<f64>().unwrap_or(0.0);
                        if peak <= 0.0 {
                            0.0
                        } else {
                            (number / peak).clamp(0.0, 1.0)
                        }
                    })
                    .collect();
                self.labels = current
                    .values
                    .iter()
                    .map(|(_, value)| value.clone())
                    .collect();
                let latest = current
                    .values
                    .last()
                    .map_or("—", |(_, value)| value.as_str());
                self.summary = format!(
                    "{} · {} readings · latest {}",
                    current.key,
                    current.values.len(),
                    latest
                );
            }
            None => {
                self.points.clear();
                self.labels.clear();
                self.summary = "No numeric history yet".into();
            }
        }
    }

    pub fn select(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.series.len() {
            self.selected = index as usize;
            self.recompute();
        }
    }

    pub fn keys(&self) -> Vec<SharedString> {
        self.series
            .iter()
            .map(|series| SharedString::from(series.key.as_str()))
            .collect()
    }

    pub fn points(&self) -> &[f64] {
        &self.points
    }

    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn preview(&self) -> &str {
        &self.preview
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }

    pub fn pending_delete(&self) -> bool {
        self.pending_delete
    }

    pub fn units(&self) -> Units {
        self.units
    }

    /// Build a JSON export in the preview; nothing is written to disk here.
    pub fn export_json(&mut self, state: &AppState) {
        self.build_export(state.export_json(), "JSON");
    }

    /// Build a CSV export in the preview; nothing is written to disk here.
    pub fn export_csv(&mut self, state: &AppState) {
        self.build_export(state.export_csv(), "CSV");
    }

    fn build_export(&mut self, result: Result<String, AppError>, kind: &str) {
        match result {
            Ok(text) if !text.is_empty() => {
                self.preview = text;
                self.status = format!("{kind} export ready — copy it from the preview");
            }
            Ok(_) => {
                self.preview.clear();
                self.status = "No local store configured".into();
            }
            Err(error) => {
                self.preview.clear();
                self.status = error.to_string();
            }
        }
    }

    /// First step: show the warning; nothing is erased yet.
    pub fn request_delete(&mut self) {
        self.pending_delete = true;
        self.status = "Delete requested — confirm to erase all local data".into();
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete = false;
        self.status = "Delete cancelled".into();
    }

    /// Second, explicit step: erase only after confirmation.
    pub fn delete_all(&mut self, state: &mut AppState) -> Result<(), AppError> {
        if !self.pending_delete {
            return Ok(());
        }
        state.delete_all()?;
        self.pending_delete = false;
        self.preview.clear();
        self.status = "All local readings and frames deleted".into();
        self.reload(state)?;
        Ok(())
    }

    pub fn toggle_units(&mut self, state: &mut AppState) -> Result<(), AppError> {
        self.units = self.units.toggle();
        state.set_units(self.units.label())?;
        self.status = format!("Display units set to {}", self.units.label());
        Ok(())
    }

    /// Copy the current state into the generated window (this crate only).
    pub fn apply_to(&self, window: &AppWindow) {
        window.set_data_series(ModelRc::new(VecModel::from(self.keys())));
        window.set_data_points(ModelRc::new(VecModel::from(
            self.points
                .iter()
                .map(|value| *value as f32)
                .collect::<Vec<_>>(),
        )));
        window.set_data_point_values(ModelRc::new(VecModel::from(
            self.labels
                .iter()
                .map(|label| SharedString::from(label.as_str()))
                .collect::<Vec<_>>(),
        )));
        window.set_data_selected_series(self.selected as i32);
        window.set_data_series_summary(SharedString::from(self.summary.as_str()));
        window.set_data_preview(SharedString::from(self.preview.as_str()));
        window.set_data_status(SharedString::from(self.status.as_str()));
        window.set_data_pending_delete(self.pending_delete);
        window.set_data_units(SharedString::from(self.units.label()));
    }
}

#[cfg(test)]
mod tests {
    use openwatch_core::store::Store;

    use super::*;

    #[test]
    fn units_toggle_round_trips() {
        let mut view = DataView::default();
        assert_eq!(view.units(), Units::Metric);
        let state = &mut AppState::default().with_store(Store::open(":memory:").unwrap());
        view.toggle_units(state).unwrap();
        assert_eq!(view.units(), Units::Imperial);
        assert_eq!(state.units().as_deref(), Some("imperial"));
        view.toggle_units(state).unwrap();
        assert_eq!(view.units(), Units::Metric);
    }

    #[test]
    fn history_chart_skips_text_and_normalizes_values() {
        let mut store = Store::open(":memory:").unwrap();
        store.put_reading("steps", "50", 1).unwrap();
        store.put_reading("steps", "100", 2).unwrap();
        store.put_reading("language", "international", 1).unwrap();
        let mut view = DataView::default();
        let state = AppState::default().with_store(store);
        view.reload(&state).unwrap();
        assert_eq!(view.keys().len(), 1, "only numeric series are charted");
        assert_eq!(view.keys()[0].as_str(), "steps");
        assert_eq!(view.points(), [0.5, 1.0]);
        assert_eq!(view.labels(), ["50", "100"]);
        assert!(view.summary().contains("latest 100"));
    }

    #[test]
    fn exports_build_a_preview_without_a_store_path() {
        let mut store = Store::open(":memory:").unwrap();
        store.put_reading("steps", "100", 1).unwrap();
        let state = AppState::default().with_store(store);
        let mut view = DataView::default();
        view.export_json(&state);
        assert!(view.preview().contains("steps"));
        assert!(view.status().contains("JSON"));
        view.export_csv(&state);
        assert!(view.preview().contains("key,value,at_unix"));
        assert!(view.status().contains("CSV"));
    }

    #[test]
    fn delete_is_a_two_step_action() {
        let mut store = Store::open(":memory:").unwrap();
        store.put_reading("steps", "100", 1).unwrap();
        let mut state = AppState::default().with_store(store);
        let mut view = DataView::default();

        view.delete_all(&mut state).unwrap();
        assert_eq!(state.reading_keys().unwrap().len(), 1, "no silent delete");

        view.request_delete();
        assert!(view.pending_delete());
        view.delete_all(&mut state).unwrap();
        assert!(!view.pending_delete());
        assert!(state.reading_keys().unwrap().is_empty());
        assert_eq!(view.status(), "All local readings and frames deleted");
    }
}

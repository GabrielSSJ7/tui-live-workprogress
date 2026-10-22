//! Application state: which view is open, what is selected, and the loaded world.
//! Rendering reads it; `input` drives the transitions defined here.

use std::path::PathBuf;

use crate::loader::{self, World};
use crate::map_grid;
use crate::model::{Demand, Project};

/// Zoom levels: all realms, one realm's quests, one quest's stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Overworld,
    Realm,
    Dungeon,
}

pub struct App {
    pub root: PathBuf,
    pub world: World,
    pub view: View,
    pub project_idx: usize,
    pub demand_idx: usize,
    pub stage_idx: usize,
    /// Overworld columns at the current terminal width, for up/down moves.
    pub map_cols: usize,
    /// Animation tick, advanced by the event loop.
    pub frame: u64,
    pub should_quit: bool,
    fingerprint: u64,
}

impl App {
    pub fn new(root: PathBuf) -> App {
        App {
            world: loader::load_world(&root),
            fingerprint: loader::fingerprint(&root),
            root,
            view: View::Overworld,
            project_idx: 0,
            demand_idx: 0,
            stage_idx: 0,
            map_cols: 1,
            frame: 0,
            should_quit: false,
        }
    }

    pub fn project(&self) -> Option<&Project> {
        self.world.projects.get(self.project_idx)
    }

    pub fn demand(&self) -> Option<&Demand> {
        self.project()?.demands.get(self.demand_idx)
    }

    /// Reload only when something on disk changed; returns whether it did.
    pub fn refresh_if_changed(&mut self) -> bool {
        let current = loader::fingerprint(&self.root);
        if current == self.fingerprint {
            return false;
        }
        self.reload();
        true
    }

    /// Re-read everything, keeping the selection on the same realm and quest.
    pub fn reload(&mut self) {
        let project_slug = self.project().map(|p| p.slug.clone());
        let demand_id = self.demand().map(|d| d.id.clone());
        self.world = loader::load_world(&self.root);
        self.fingerprint = loader::fingerprint(&self.root);
        self.project_idx = self.position_of_project(project_slug.as_deref());
        self.demand_idx = self.position_of_demand(demand_id.as_deref());
        self.clamp_selection();
    }

    fn position_of_project(&self, slug: Option<&str>) -> usize {
        let found = slug.and_then(|s| self.world.projects.iter().position(|p| p.slug == s));
        found.unwrap_or(self.project_idx)
    }

    fn position_of_demand(&self, id: Option<&str>) -> usize {
        let demands = self
            .project()
            .map(|p| p.demands.as_slice())
            .unwrap_or_default();
        let found = id.and_then(|id| demands.iter().position(|d| d.id == id));
        found.unwrap_or(self.demand_idx)
    }

    fn clamp_selection(&mut self) {
        self.project_idx = self
            .project_idx
            .min(self.world.projects.len().saturating_sub(1));
        let demands = self.project().map_or(0, |p| p.demands.len());
        self.demand_idx = self.demand_idx.min(demands.saturating_sub(1));
        let stages = self.demand().map_or(0, |d| d.stages.len());
        self.stage_idx = self.stage_idx.min(stages.saturating_sub(1));
    }

    /// Zoom in one level; the dungeon opens on the stage being fought.
    pub fn enter(&mut self) {
        match self.view {
            View::Overworld if self.project().is_some() => {
                self.view = View::Realm;
                self.demand_idx = 0;
            }
            View::Realm if self.demand().is_some() => {
                self.view = View::Dungeon;
                self.stage_idx = self.demand().and_then(Demand::current_stage).unwrap_or(0);
            }
            _ => {}
        }
    }

    pub fn back(&mut self) {
        self.view = match self.view {
            View::Dungeon => View::Realm,
            _ => View::Overworld,
        };
    }

    /// Arrow-key movement, interpreted per view.
    pub fn move_by(&mut self, dx: i64, dy: i64) {
        match self.view {
            View::Overworld => {
                let len = self.world.projects.len();
                self.project_idx = map_grid::step(self.project_idx, len, self.map_cols, dx, dy);
            }
            View::Realm if dx != 0 => self.switch_realm(dx),
            View::Realm => self.demand_idx = offset(self.demand_idx, dy, self.demand_count()),
            View::Dungeon => self.stage_idx = offset(self.stage_idx, dx + dy, self.stage_count()),
        }
    }

    /// Left/right inside a realm walks to the neighbouring realm.
    fn switch_realm(&mut self, dx: i64) {
        self.project_idx = offset(self.project_idx, dx, self.world.projects.len());
        self.demand_idx = 0;
    }

    fn demand_count(&self) -> usize {
        self.project().map_or(0, |p| p.demands.len())
    }

    fn stage_count(&self) -> usize {
        self.demand().map_or(0, |d| d.stages.len())
    }
}

/// `index + delta`, clamped to `0..len`.
fn offset(index: usize, delta: i64, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (index as i64 + delta).clamp(0, len as i64 - 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::run_in;
    use crate::scratch;

    fn seeded(tag: &str) -> App {
        let root = scratch::dir(tag);
        crate::demo::seed(&root).unwrap();
        App::new(root)
    }

    #[test]
    fn enter_and_back_walk_the_zoom_levels() {
        let mut app = seeded("app-zoom");
        app.project_idx = 1;
        app.enter();
        assert_eq!(app.view, View::Realm);
        app.move_by(0, 1);
        assert_eq!(app.demand().unwrap().id, "DB-10");
        app.enter();
        assert_eq!((app.view, app.stage_idx), (View::Dungeon, 2));
        app.move_by(1, 0);
        assert_eq!(app.stage_idx, 3);
        app.move_by(1, 0);
        assert_eq!(app.stage_idx, 3);
        app.back();
        assert_eq!(app.view, View::Realm);
        app.back();
        assert_eq!(app.view, View::Overworld);
    }

    #[test]
    fn overworld_moves_follow_the_grid_and_realm_sides_switch_realm() {
        let mut app = seeded("app-grid");
        app.map_cols = 3;
        // Row 1 runs right→left and holds only indices 3 (col 2) and 4 (col 1).
        app.move_by(0, 1);
        assert_eq!(app.project_idx, 4);
        app.enter();
        app.move_by(-1, 0);
        assert_eq!((app.project_idx, app.demand_idx), (3, 0));
    }

    #[test]
    fn reload_keeps_selection_by_id() {
        let mut app = seeded("app-reload");
        app.project_idx = 2;
        assert!(!app.refresh_if_changed());
        let args: Vec<String> = ["realm", "aaa-first", "Aaa First", "0"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        run_in(&app.root, &args).unwrap();
        assert!(app.refresh_if_changed());
        assert_eq!(app.project().unwrap().slug, "frontend-realm");
        assert_eq!(app.project_idx, 3);
    }

    #[test]
    fn empty_world_is_safe() {
        let mut app = App::new(scratch::dir("app-empty"));
        app.enter();
        app.move_by(1, 1);
        assert_eq!((app.view, app.project_idx), (View::Overworld, 0));
        assert!(app.demand().is_none());
        assert_eq!(offset(0, 5, 0), 0);
    }
}

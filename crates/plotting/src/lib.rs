//! Plotlines, ordered plot points, and optional links to manuscript scenes.
//!
//! The grid is local project data. Storage writes it to SQLite; the desktop
//! workspace edits it. A plot point may name a binder scene, and that link is
//! only a reference — the scene text stays in the document table.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use writing_studio_core::{Id, Timestamp};

const LINE_COLORS: [&str; 5] = ["gold", "green", "blue", "red", "violet"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plotline {
    pub id: Id,
    pub title: String,
    pub color: String,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Plotline {
    pub fn new(title: impl Into<String>, sort_index: i64, color: impl Into<String>) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            title: title.into(),
            color: color.into(),
            sort_index,
            created_at: now,
            updated_at: now,
        }
    }

    fn touch(&mut self) {
        self.updated_at = Timestamp::now();
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlotPoint {
    pub id: Id,
    pub plotline_id: Id,
    pub title: String,
    pub summary: String,
    pub scene_id: Option<Id>,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl PlotPoint {
    pub fn new(plotline_id: Id, title: impl Into<String>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            plotline_id,
            title: title.into(),
            summary: String::new(),
            scene_id: None,
            sort_index,
            created_at: now,
            updated_at: now,
        }
    }

    fn touch(&mut self) {
        self.updated_at = Timestamp::now();
    }
}

/// Every plotline and plot point in one project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlotGrid {
    pub lines: Vec<Plotline>,
    pub points: Vec<PlotPoint>,
}

impl PlotGrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lines_in_order(&self) -> Vec<&Plotline> {
        let mut lines: Vec<&Plotline> = self.lines.iter().collect();
        lines.sort_by_key(|line| (line.sort_index, line.id));
        lines
    }

    pub fn points_on(&self, line_id: Id) -> Vec<&PlotPoint> {
        let mut points: Vec<&PlotPoint> = self
            .points
            .iter()
            .filter(|point| point.plotline_id == line_id)
            .collect();
        points.sort_by_key(|point| (point.sort_index, point.id));
        points
    }

    pub fn add_line(&mut self) -> Id {
        let sort_index = self
            .lines
            .iter()
            .map(|line| line.sort_index)
            .max()
            .map(|index| index + 1)
            .unwrap_or(0);
        let color = LINE_COLORS[self.lines.len() % LINE_COLORS.len()];
        let title = format!("Plotline {}", self.lines.len() + 1);
        let line = Plotline::new(title, sort_index, color);
        let id = line.id;
        self.lines.push(line);
        id
    }

    pub fn set_line_title(&mut self, id: Id, title: String) -> bool {
        let Some(line) = self.lines.iter_mut().find(|line| line.id == id) else {
            return false;
        };
        if line.title == title {
            return false;
        }
        line.title = title;
        line.touch();
        true
    }

    pub fn set_line_color(&mut self, id: Id, color: String) -> bool {
        let Some(line) = self.lines.iter_mut().find(|line| line.id == id) else {
            return false;
        };
        if line.color == color {
            return false;
        }
        line.color = color;
        line.touch();
        true
    }

    pub fn remove_line(&mut self, id: Id) -> bool {
        let before = self.lines.len();
        self.lines.retain(|line| line.id != id);
        if self.lines.len() == before {
            return false;
        }
        self.points.retain(|point| point.plotline_id != id);
        true
    }

    pub fn add_point(&mut self, line_id: Id) -> Option<Id> {
        if !self.lines.iter().any(|line| line.id == line_id) {
            return None;
        }
        let sort_index = self
            .points
            .iter()
            .filter(|point| point.plotline_id == line_id)
            .map(|point| point.sort_index)
            .max()
            .map(|index| index + 1)
            .unwrap_or(0);
        let point = PlotPoint::new(line_id, "Plot point", sort_index);
        let id = point.id;
        self.points.push(point);
        Some(id)
    }

    pub fn set_point_title(&mut self, id: Id, title: String) -> bool {
        let Some(point) = self.points.iter_mut().find(|point| point.id == id) else {
            return false;
        };
        if point.title == title {
            return false;
        }
        point.title = title;
        point.touch();
        true
    }

    pub fn set_point_summary(&mut self, id: Id, summary: String) -> bool {
        let Some(point) = self.points.iter_mut().find(|point| point.id == id) else {
            return false;
        };
        if point.summary == summary {
            return false;
        }
        point.summary = summary;
        point.touch();
        true
    }

    pub fn set_point_scene(&mut self, id: Id, scene_id: Option<Id>) -> bool {
        let Some(point) = self.points.iter_mut().find(|point| point.id == id) else {
            return false;
        };
        if point.scene_id == scene_id {
            return false;
        }
        point.scene_id = scene_id;
        point.touch();
        true
    }

    pub fn remove_point(&mut self, id: Id) -> bool {
        let Some(position) = self.points.iter().position(|point| point.id == id) else {
            return false;
        };
        let line_id = self.points[position].plotline_id;
        self.points.remove(position);
        self.reindex_points(line_id);
        true
    }

    /// Move a plot point one step earlier or later on its plotline.
    pub fn shift_point(&mut self, id: Id, earlier: bool) -> bool {
        let Some(point) = self.points.iter().find(|point| point.id == id) else {
            return false;
        };
        let line_id = point.plotline_id;
        let mut order = self.point_order(line_id);
        let Some(index) = order.iter().position(|item| *item == id) else {
            return false;
        };
        let neighbor = if earlier {
            index.checked_sub(1)
        } else if index + 1 < order.len() {
            Some(index + 1)
        } else {
            None
        };
        let Some(neighbor) = neighbor else {
            return false;
        };
        order.swap(index, neighbor);
        self.write_point_order(&order);
        true
    }

    /// Drop links that point at binder items which no longer exist.
    pub fn detach_scenes(&mut self, scene_ids: &[Id]) -> bool {
        let gone: HashSet<Id> = scene_ids.iter().copied().collect();
        let mut changed = false;
        for point in &mut self.points {
            if point.scene_id.is_some_and(|id| gone.contains(&id)) {
                point.scene_id = None;
                point.touch();
                changed = true;
            }
        }
        changed
    }

    fn point_order(&self, line_id: Id) -> Vec<Id> {
        self.points_on(line_id).into_iter().map(|point| point.id).collect()
    }

    fn write_point_order(&mut self, order: &[Id]) {
        for (index, id) in order.iter().enumerate() {
            if let Some(point) = self.points.iter_mut().find(|point| point.id == *id) {
                let sort_index = index as i64;
                if point.sort_index != sort_index {
                    point.sort_index = sort_index;
                    point.touch();
                }
            }
        }
    }

    fn reindex_points(&mut self, line_id: Id) {
        let order = self.point_order(line_id);
        self.write_point_order(&order);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plotline_holds_ordered_points_linked_to_scenes() {
        let mut grid = PlotGrid::new();
        let line = grid.add_line();
        assert_eq!(grid.lines[0].title, "Plotline 1");
        assert_eq!(grid.lines[0].color, "gold");

        let first = grid.add_point(line).unwrap();
        let second = grid.add_point(line).unwrap();
        let scene = Id::new();
        assert!(grid.set_point_scene(second, Some(scene)));
        assert!(grid.shift_point(second, true));

        let order = grid.points_on(line);
        assert_eq!(order[0].id, second);
        assert_eq!(order[0].scene_id, Some(scene));
        assert_eq!(order[1].id, first);
        assert!(!grid.shift_point(second, true));
    }

    #[test]
    fn removing_a_plotline_removes_its_points() {
        let mut grid = PlotGrid::new();
        let kept = grid.add_line();
        let dropped = grid.add_line();
        grid.add_point(kept).unwrap();
        let point = grid.add_point(dropped).unwrap();
        assert!(grid.remove_line(dropped));
        assert!(grid.points.iter().all(|item| item.id != point));
        assert_eq!(grid.lines.len(), 1);
        assert!(grid.points_on(kept).len() == 1);
    }

    #[test]
    fn emptying_scenes_clears_only_those_links() {
        let mut grid = PlotGrid::new();
        let line = grid.add_line();
        let linked = grid.add_point(line).unwrap();
        let other = grid.add_point(line).unwrap();
        let gone = Id::new();
        let stays = Id::new();
        grid.set_point_scene(linked, Some(gone));
        grid.set_point_scene(other, Some(stays));
        assert!(grid.detach_scenes(&[gone, Id::new()]));
        assert_eq!(grid.points.iter().find(|point| point.id == linked).unwrap().scene_id, None);
        assert_eq!(
            grid.points.iter().find(|point| point.id == other).unwrap().scene_id,
            Some(stays)
        );
        assert!(!grid.detach_scenes(&[gone]));
    }
}

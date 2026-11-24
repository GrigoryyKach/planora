//! Subtasks: a task can have subtasks, one level deep. A subtask is a full task (same fields, same editor);
//! it only adds `parent_id` and shares the parent's profile and project. The order of the subtasks of one
//! parent is their order in `Data::tasks`.

use super::calendar::set_status;
use super::types::{Data, Id, Profile, Status, Task};
use chrono::NaiveDateTime;
use std::collections::HashSet;

/// Why a task cannot become a subtask of another one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SubtaskError {
    NoSuchTask,
    NoSuchParent,
    /// A task cannot be its own subtask.
    SameTask,
    /// Depth is one level: a subtask cannot have subtasks.
    ParentIsSubtask,
    /// A task that has subtasks of its own cannot become a subtask.
    HasSubtasks,
    /// Repeating subtasks are not supported (the series would have no clear meaning inside a parent).
    Recurring,
}

impl Data {
    /// Subtasks of `parent`, in their order.
    pub fn subtasks_of(&self, parent: &str) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.parent_id.as_deref() == Some(parent)).collect()
    }

    pub fn has_subtasks(&self, id: &str) -> bool {
        self.tasks.iter().any(|t| t.parent_id.as_deref() == Some(id))
    }

    /// The parent of a subtask.
    pub fn parent_of(&self, task: &Task) -> Option<&Task> {
        task.parent_id.as_deref().and_then(|p| self.task(p))
    }

    /// `(done, total)` of the subtasks. Cancelled subtasks are left out of both numbers.
    pub fn subtask_progress(&self, parent: &str) -> (usize, usize) {
        let subs = self.subtasks_of(parent);
        let total = subs.iter().filter(|s| !s.is_cancelled()).count();
        let done = subs.iter().filter(|s| s.status == Some(Status::Done)).count();
        (done, total)
    }

    /// Subtasks that are neither done nor cancelled.
    pub fn open_subtask_count(&self, parent: &str) -> usize {
        self.subtasks_of(parent).iter().filter(|s| !s.is_closed()).count()
    }

    /// Every subtask is done (there is at least one). The parent is *not* completed automatically; the UI only
    /// suggests it.
    pub fn all_subtasks_done(&self, parent: &str) -> bool {
        let (done, total) = self.subtask_progress(parent);
        total > 0 && done == total
    }

    /// Can `id` become a subtask of `parent`? (The depth guard.)
    pub fn can_make_subtask(&self, id: &str, parent: &str) -> Result<(), SubtaskError> {
        let task = self.task(id).ok_or(SubtaskError::NoSuchTask)?;
        let p = self.task(parent).ok_or(SubtaskError::NoSuchParent)?;
        if id == parent {
            return Err(SubtaskError::SameTask);
        }
        if p.parent_id.is_some() {
            return Err(SubtaskError::ParentIsSubtask);
        }
        if self.has_subtasks(id) {
            return Err(SubtaskError::HasSubtasks);
        }
        if task.recurrence.is_some() {
            return Err(SubtaskError::Recurring);
        }
        Ok(())
    }

    /// Where a new or moved subtask of `parent` goes: right after its last sibling, else right after the parent.
    fn subtask_slot(&self, parent: &str) -> usize {
        let last_sibling = self.tasks.iter().rposition(|t| t.parent_id.as_deref() == Some(parent));
        last_sibling.or_else(|| self.tasks.iter().position(|t| t.id == parent)).map_or(self.tasks.len(), |i| i + 1)
    }

    /// Turns a task into the last subtask of `parent`. The task takes the parent's profile and project.
    pub fn make_subtask(&mut self, id: &str, parent: &str, now: NaiveDateTime) -> Result<(), SubtaskError> {
        self.can_make_subtask(id, parent)?;
        let pos = self.tasks.iter().position(|t| t.id == id).ok_or(SubtaskError::NoSuchTask)?;
        let mut task = self.tasks.remove(pos);
        let (profile, project) = {
            let p = self.task(parent).ok_or(SubtaskError::NoSuchParent)?;
            (p.profile, p.project_id.clone())
        };
        task.parent_id = Some(parent.to_string());
        task.profile = profile;
        task.project_id = project;
        task.updated = now;
        let slot = self.subtask_slot(parent);
        self.tasks.insert(slot.min(self.tasks.len()), task);
        Ok(())
    }

    /// Turns a subtask back into a normal task (it keeps its profile and project). `false` if it is not a subtask.
    pub fn convert_to_task(&mut self, id: &str, now: NaiveDateTime) -> bool {
        match self.task_mut(id) {
            Some(t) if t.parent_id.is_some() => {
                t.parent_id = None;
                t.updated = now;
                true
            }
            _ => false,
        }
    }

    /// Adds a subtask at the end of the parent's subtasks. `None` if the parent is missing or is a subtask itself.
    pub fn add_subtask(&mut self, parent: &str, title: &str, now: NaiveDateTime) -> Option<Id> {
        let p = self.task(parent).filter(|p| p.parent_id.is_none())?;
        let mut t = Task::new(p.profile, title, now);
        t.project_id = p.project_id.clone();
        t.parent_id = Some(parent.to_string());
        let id = t.id.clone();
        let slot = self.subtask_slot(parent);
        self.tasks.insert(slot.min(self.tasks.len()), t);
        Some(id)
    }

    /// Moves the subtask `id` to position `to` among the subtasks of `parent` (0 = first). Other tasks do not move.
    pub fn move_subtask(&mut self, parent: &str, id: &str, to: usize) -> bool {
        let slots: Vec<usize> = self.tasks.iter().enumerate().filter(|(_, t)| t.parent_id.as_deref() == Some(parent)).map(|(i, _)| i).collect();
        let Some(from) = slots.iter().position(|&i| self.tasks[i].id == id) else { return false };
        let to = to.min(slots.len() - 1);
        if from == to {
            return true;
        }
        let mut items: Vec<Task> = slots.iter().map(|&i| self.tasks[i].clone()).collect();
        let item = items.remove(from);
        items.insert(to, item);
        for (slot, item) in slots.into_iter().zip(items) {
            self.tasks[slot] = item;
        }
        true
    }

    /// Copies the parent's profile and project to its subtasks (call after the parent moved).
    pub fn sync_children(&mut self, parent: &str) {
        let Some((profile, project)) = self.task(parent).map(|p| (p.profile, p.project_id.clone())) else { return };
        for t in self.tasks.iter_mut().filter(|t| t.parent_id.as_deref() == Some(parent)) {
            t.profile = profile;
            t.project_id = project.clone();
        }
    }

    /// Moves a top-level task, and with it its subtasks, to another profile or project. A subtask cannot be moved
    /// on its own (it lives where its parent lives): returns `false` then.
    pub fn set_location(&mut self, id: &str, profile: Profile, project: Option<String>, now: NaiveDateTime) -> bool {
        match self.task_mut(id) {
            Some(t) if t.parent_id.is_none() => {
                t.profile = profile;
                t.project_id = project;
                t.updated = now;
            }
            _ => return false,
        }
        self.sync_children(id);
        true
    }

    /// Marks the task done together with all its open subtasks.
    pub fn complete_with_subtasks(&mut self, id: &str, now: NaiveDateTime) {
        for t in self.tasks.iter_mut().filter(|t| t.parent_id.as_deref() == Some(id) && !t.is_closed()) {
            set_status(t, Some(Status::Done), now);
        }
        if let Some(t) = self.task_mut(id) {
            set_status(t, Some(Status::Done), now);
        }
    }

    /// Marks only the task done and leaves its subtasks as they are.
    pub fn complete_only(&mut self, id: &str, now: NaiveDateTime) {
        if let Some(t) = self.task_mut(id) {
            set_status(t, Some(Status::Done), now);
        }
    }

    /// All tasks that match a search text (title or description), subtasks included.
    pub fn search(&self, query: &str) -> Vec<&Task> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        self.tasks.iter().filter(|x| x.title.to_lowercase().contains(&q) || x.description.to_lowercase().contains(&q)).collect()
    }

    /// Repairs the parent links after loading or importing: a subtask whose parent is missing, is itself a
    /// subtask, or is the task itself becomes a normal task, and every subtask takes its parent's location.
    pub fn normalize(&mut self) {
        let top: HashSet<Id> = self.tasks.iter().filter(|t| t.parent_id.is_none()).map(|t| t.id.clone()).collect();
        for t in &mut self.tasks {
            if t.parent_id.as_ref().is_some_and(|p| *p == t.id || !top.contains(p)) {
                t.parent_id = None;
            }
        }
        let parents: Vec<Id> = self.tasks.iter().filter(|t| t.parent_id.is_some()).filter_map(|t| t.parent_id.clone()).collect::<HashSet<_>>().into_iter().collect();
        for p in parents {
            self.sync_children(&p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::{RecKind, Recurrence};
    use chrono::NaiveDate;

    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 24).unwrap().and_hms_opt(9, 0, 0).unwrap()
    }
    fn task(data: &mut Data, title: &str) -> Id {
        let t = Task::new(Profile::Personal, title, now());
        let id = t.id.clone();
        data.tasks.push(t);
        id
    }
    fn titles(data: &Data, parent: &str) -> Vec<String> {
        data.subtasks_of(parent).iter().map(|t| t.title.clone()).collect()
    }

    #[test]
    fn depth_is_one_level() {
        let mut d = Data::default();
        let a = task(&mut d, "A");
        let b = task(&mut d, "B");
        let c = task(&mut d, "C");
        assert_eq!(d.make_subtask(&b, &a, now()), Ok(()));
        // A subtask cannot become a parent, and a parent cannot become a subtask.
        assert_eq!(d.can_make_subtask(&c, &b), Err(SubtaskError::ParentIsSubtask));
        assert_eq!(d.make_subtask(&c, &b, now()), Err(SubtaskError::ParentIsSubtask));
        assert_eq!(d.can_make_subtask(&a, &c), Err(SubtaskError::HasSubtasks));
        assert_eq!(d.can_make_subtask(&a, &a), Err(SubtaskError::SameTask));
        assert_eq!(d.can_make_subtask("nope", &a), Err(SubtaskError::NoSuchTask));
        assert_eq!(d.can_make_subtask(&c, "nope"), Err(SubtaskError::NoSuchParent));
        // add_subtask refuses a subtask as the parent too.
        assert!(d.add_subtask(&b, "deeper", now()).is_none());
        assert!(d.add_subtask(&a, "fine", now()).is_some());
        assert!(d.tasks.iter().all(|t| t.parent_id.as_ref().is_none_or(|p| d.task(p).unwrap().parent_id.is_none())));
    }

    #[test]
    fn recurring_tasks_cannot_become_subtasks() {
        let mut d = Data::default();
        let a = task(&mut d, "A");
        let r = task(&mut d, "Repeating");
        d.task_mut(&r).unwrap().recurrence = Some(Recurrence { kind: RecKind::Daily, interval: 1, until: None });
        assert_eq!(d.make_subtask(&r, &a, now()), Err(SubtaskError::Recurring));
        assert!(d.task(&r).unwrap().parent_id.is_none());
    }

    #[test]
    fn convert_to_subtask_and_back_keeps_everything() {
        let mut d = Data::default();
        d.projects.push(super::super::types::Project {
            id: "p1".into(),
            profile: Profile::Work,
            name: "Video".into(),
            description: String::new(),
            color: 0,
            archived: false,
            created: now(),
        });
        let parent = task(&mut d, "Make video");
        {
            let p = d.task_mut(&parent).unwrap();
            p.profile = Profile::Work;
            p.project_id = Some("p1".into());
        }
        let other = task(&mut d, "Write script");
        {
            let t = d.task_mut(&other).unwrap();
            t.description = "notes".into();
            t.scheduled = NaiveDate::from_ymd_opt(2026, 11, 25);
            t.checklist.push(crate::model::types::CheckItem { text: "x".into(), done: true });
            t.priority = Some(crate::model::types::Priority::High);
        }
        let before = d.task(&other).unwrap().clone();
        d.make_subtask(&other, &parent, now()).unwrap();
        let sub = d.task(&other).unwrap();
        assert_eq!(sub.parent_id.as_deref(), Some(parent.as_str()));
        assert_eq!((sub.profile, sub.project_id.as_deref()), (Profile::Work, Some("p1")));
        assert_eq!(sub.description, "notes");
        assert_eq!(sub.scheduled, before.scheduled);
        assert_eq!(sub.checklist, before.checklist);
        assert!(d.convert_to_task(&other, now()));
        let back = d.task(&other).unwrap();
        assert!(back.parent_id.is_none());
        // Everything but the location, the parent link and the timestamp is unchanged.
        assert_eq!((back.title.as_str(), back.description.as_str(), back.scheduled, back.priority), (before.title.as_str(), "notes", before.scheduled, before.priority));
        assert_eq!(back.project_id.as_deref(), Some("p1"));
        // Converting a normal task does nothing.
        assert!(!d.convert_to_task(&parent, now()));
    }

    #[test]
    fn moving_the_parent_moves_its_subtasks() {
        let mut d = Data::default();
        let a = task(&mut d, "A");
        let s1 = d.add_subtask(&a, "s1", now()).unwrap();
        let s2 = d.add_subtask(&a, "s2", now()).unwrap();
        assert!(d.set_location(&a, Profile::Work, Some("proj".into()), now()));
        for id in [&a, &s1, &s2] {
            let t = d.task(id).unwrap();
            assert_eq!((t.profile, t.project_id.as_deref()), (Profile::Work, Some("proj")));
        }
        // A subtask cannot be moved away from its parent.
        assert!(!d.set_location(&s1, Profile::Personal, None, now()));
        assert_eq!(d.task(&s1).unwrap().project_id.as_deref(), Some("proj"));
    }

    #[test]
    fn subtask_order_is_kept_per_parent() {
        let mut d = Data::default();
        let a = task(&mut d, "A");
        let b = task(&mut d, "B");
        for t in ["a1", "a2", "a3"] {
            d.add_subtask(&a, t, now());
        }
        for t in ["b1", "b2"] {
            d.add_subtask(&b, t, now());
        }
        assert_eq!(titles(&d, &a), ["a1", "a2", "a3"]);
        let a3 = d.subtasks_of(&a)[2].id.clone();
        assert!(d.move_subtask(&a, &a3, 0));
        assert_eq!(titles(&d, &a), ["a3", "a1", "a2"]);
        // The other parent and the top-level order did not change.
        assert_eq!(titles(&d, &b), ["b1", "b2"]);
        let top: Vec<_> = d.tasks.iter().filter(|t| t.parent_id.is_none()).map(|t| t.title.as_str()).collect();
        assert_eq!(top, ["A", "B"]);
        // Moving past the end clamps; an unknown id is refused.
        let a3 = d.subtasks_of(&a)[0].id.clone();
        assert!(d.move_subtask(&a, &a3, 99));
        assert_eq!(titles(&d, &a), ["a1", "a2", "a3"]);
        assert!(!d.move_subtask(&a, &b, 0));
    }

    #[test]
    fn progress_and_completion_rules() {
        let mut d = Data::default();
        let a = task(&mut d, "A");
        let ids: Vec<Id> = (0..4).map(|i| d.add_subtask(&a, &format!("s{i}"), now()).unwrap()).collect();
        assert_eq!(d.subtask_progress(&a), (0, 4));
        assert!(!d.all_subtasks_done(&a));
        set_status(d.task_mut(&ids[0]).unwrap(), Some(Status::Done), now());
        set_status(d.task_mut(&ids[1]).unwrap(), Some(Status::Cancelled), now());
        // Cancelled subtasks are not counted at all.
        assert_eq!(d.subtask_progress(&a), (1, 3));
        assert_eq!(d.open_subtask_count(&a), 2);
        // Finishing the last open subtask does not complete the parent.
        for id in &ids[2..] {
            set_status(d.task_mut(id).unwrap(), Some(Status::Done), now());
        }
        assert!(d.all_subtasks_done(&a));
        assert_eq!(d.task(&a).unwrap().status, None);
        // No subtasks, no "all done".
        let lone = task(&mut d, "lone");
        assert!(!d.all_subtasks_done(&lone));
    }

    #[test]
    fn completing_the_parent_asks_and_applies_either_choice() {
        let mut d = Data::default();
        let a = task(&mut d, "A");
        let s1 = d.add_subtask(&a, "s1", now()).unwrap();
        let s2 = d.add_subtask(&a, "s2", now()).unwrap();
        set_status(d.task_mut(&s2).unwrap(), Some(Status::Cancelled), now());
        let mut only = d.clone();
        only.complete_only(&a, now());
        assert_eq!(only.task(&a).unwrap().status, Some(Status::Done));
        assert_eq!(only.task(&s1).unwrap().status, None);
        assert_eq!(only.open_subtask_count(&a), 1);
        d.complete_with_subtasks(&a, now());
        assert_eq!(d.task(&a).unwrap().status, Some(Status::Done));
        assert_eq!(d.task(&s1).unwrap().status, Some(Status::Done));
        assert!(d.task(&s1).unwrap().completed_at.is_some());
        // A cancelled subtask stays cancelled.
        assert_eq!(d.task(&s2).unwrap().status, Some(Status::Cancelled));
    }

    #[test]
    fn normalize_repairs_broken_links() {
        let mut d = Data::default();
        let a = task(&mut d, "A");
        let b = task(&mut d, "B");
        d.task_mut(&a).unwrap().project_id = Some("p".into());
        d.task_mut(&b).unwrap().parent_id = Some(a.clone());
        let orphan = task(&mut d, "orphan");
        d.task_mut(&orphan).unwrap().parent_id = Some("gone".into());
        let deep = task(&mut d, "deep");
        d.task_mut(&deep).unwrap().parent_id = Some(b.clone());
        let selfref = task(&mut d, "self");
        d.task_mut(&selfref).unwrap().parent_id = Some(selfref.clone());
        d.normalize();
        assert_eq!(d.task(&b).unwrap().parent_id.as_deref(), Some(a.as_str()));
        assert_eq!(d.task(&b).unwrap().project_id.as_deref(), Some("p"));
        for id in [&orphan, &deep, &selfref] {
            assert!(d.task(id).unwrap().parent_id.is_none());
        }
    }

    #[test]
    fn search_finds_subtasks() {
        let mut d = Data::default();
        let a = task(&mut d, "Make video");
        d.add_subtask(&a, "Write script", now());
        let hits: Vec<_> = d.search("script").iter().map(|t| t.title.clone()).collect();
        assert_eq!(hits, ["Write script"]);
        assert!(d.search("  ").is_empty());
    }
}

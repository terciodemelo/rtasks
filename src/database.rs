use crate::project::*;
use chrono::prelude::Utc;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::slice::Iter;

pub struct Database {
    projects: Vec<Project>,
}

impl Database {
    fn storage() -> Result<String> {
        match dirs::home_dir() {
            Some(path) => Ok(format!(
                "{}{}",
                path.to_str().unwrap(),
                "/.tasks/projects.json"
            )),
            None => Err(Error::new(
                ErrorKind::Other,
                "Couldn't resolve your home directory",
            )),
        }
    }

    pub fn load() -> Result<Database> {
        let json_data = fs::read_to_string(Database::storage()?)?;
        let projects = serde_json::from_str(json_data.as_str())?;
        Ok(Database { projects: projects })
    }

    pub fn save(&mut self) -> Result<()> {
        let content = serde_json::to_string(&self.projects)?;
        fs::write(Database::storage()?, content)
    }

    pub fn set_task_state(
        &mut self,
        project: usize,
        task: usize,
        state: State,
    ) -> Result<Option<usize>> {
        if state != self.projects[project].tasks[task].state() {
            self.projects[project].tasks[task]
                .events
                .push(Event::State {
                    data: state,
                    date_time: Utc::now(),
                });
            let task_id = self.projects[project].tasks[task].id.clone();
            self.projects[project].sort_tasks();
            self.save()?;
            Ok(self.projects[project].task_position(task_id))
        } else {
            Ok(None)
        }
    }

    pub fn add_project(&mut self, project: Project) -> Result<()> {
        self.projects.push(project);
        self.save()
    }

    pub fn add_task(&mut self, project: usize, task: Task) -> Result<Option<usize>> {
        let task_id = task.id.clone();
        self.projects[project].tasks.push(task);
        self.projects[project].sort_tasks();
        self.save()?;
        Ok(self.projects[project].task_position(task_id))
    }

    pub fn remove_project(&mut self, project: usize) -> Result<()> {
        self.projects.remove(project);
        self.save()
    }

    pub fn remove_task(&mut self, project: usize, task: usize) -> Result<()> {
        self.projects[project].tasks.remove(task);
        self.save()
    }

    pub fn swap_projects(&mut self, first: usize, second: usize) -> Result<()> {
        self.projects.swap(first, second);
        self.save()
    }

    pub fn swap_tasks(&mut self, project: usize, first: usize, second: usize) -> Result<()> {
        self.projects[project].tasks.swap(first, second);
        self.projects[project].sort_tasks();
        self.save()
    }

    pub fn projects(&self) -> Iter<Project> {
        self.projects.iter()
    }

    pub fn tasks(&self, project: usize) -> Iter<Task> {
        self.projects[project].tasks.iter()
    }

    pub fn project_count(&self) -> u16 {
        self.projects.len() as u16
    }

    pub fn task_count(&self, project: usize) -> u16 {
        self.projects[project].tasks.len() as u16
    }

    pub fn task_state(&self, project: usize, task: usize) -> State {
        self.projects[project].tasks[task].state()
    }
}

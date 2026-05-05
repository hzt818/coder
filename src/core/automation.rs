//! Automation system - scheduled recurring tasks
//!
//! Provides cron-style scheduled automation with lifecycle management.

use std::sync::Mutex;

#[allow(dead_code)]
static AUTOMATION_MANAGER: Mutex<Option<AutomationManager>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AutomationStatus {
    Active,
    Paused,
    Completed,
}

impl AutomationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AutomationStatus::Active => "active",
            AutomationStatus::Paused => "paused",
            AutomationStatus::Completed => "completed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "paused" => AutomationStatus::Paused,
            "completed" => AutomationStatus::Completed,
            _ => AutomationStatus::Active,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Automation {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub cwd: String,
    pub status: String,
    pub created_at: String,
    pub last_run: Option<String>,
}

pub struct AutomationManager {
    automations: Vec<Automation>,
    next_id: usize,
}

impl AutomationManager {
    pub fn new() -> Self {
        Self {
            automations: Vec::new(),
            next_id: 1,
        }
    }

    pub fn create(&mut self, name: &str, schedule: &str, prompt: &str) -> Automation {
        let id = format!("auto-{}", self.next_id);
        self.next_id += 1;
        let now = chrono::Utc::now().to_rfc3339();
        let auto = Automation {
            id: id.clone(),
            name: name.to_string(),
            schedule: schedule.to_string(),
            prompt: prompt.to_string(),
            cwd: ".".to_string(),
            status: AutomationStatus::Active.as_str().to_string(),
            created_at: now.clone(),
            last_run: None,
        };
        self.automations.push(auto.clone());
        auto
    }

    pub fn list(&self) -> &[Automation] {
        &self.automations
    }

    pub fn get(&self, id: &str) -> Option<&Automation> {
        self.automations.iter().find(|a| a.id == id || a.name == id)
    }

    pub fn update(
        &mut self,
        id: &str,
        name: Option<&str>,
        schedule: Option<&str>,
        prompt: Option<&str>,
        cwd: Option<&str>,
        status: Option<&str>,
    ) -> bool {
        if let Some(auto) = self.automations.iter_mut().find(|a| a.id == id || a.name == id) {
            if let Some(n) = name { auto.name = n.to_string(); }
            if let Some(s) = schedule { auto.schedule = s.to_string(); }
            if let Some(p) = prompt { auto.prompt = p.to_string(); }
            if let Some(c) = cwd { auto.cwd = c.to_string(); }
            if let Some(s) = status { auto.status = s.to_string(); }
            true
        } else {
            false
        }
    }

    pub fn set_status(&mut self, id: &str, status: AutomationStatus) -> bool {
        if let Some(auto) = self.automations.iter_mut().find(|a| a.id == id || a.name == id) {
            auto.status = status.as_str().to_string();
            true
        } else {
            false
        }
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let len = self.automations.len();
        self.automations.retain(|a| a.id != id && a.name != id);
        self.automations.len() < len
    }

    pub fn run_now(&mut self, id: &str) -> Option<String> {
        let auto = self.automations.iter_mut().find(|a| a.id == id || a.name == id)?;
        auto.last_run = Some(chrono::Utc::now().to_rfc3339());
        Some(format!("Executed automation '{}': {}", auto.name, auto.prompt))
    }

    pub fn format_list(&self) -> String {
        if self.automations.is_empty() {
            return "── Automations ──\n\nNo automations configured.".to_string();
        }
        let mut result = format!("── Automations ({}) ──\n\n", self.automations.len());
        for auto in &self.automations {
            let icon = match auto.status.as_str() {
                "active" => "▶",
                "paused" => "⏸",
                "completed" => "✅",
                _ => "❓",
            };
            result.push_str(&format!(
                "  {} {} [{}] schedule: {}\n     {}\n",
                icon, auto.name, auto.status, auto.schedule, auto.prompt
            ));
        }
        result
    }
}

impl Default for AutomationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_automation() {
        let mut mgr = AutomationManager::new();
        let auto = mgr.create("daily-test", "0 9 * * *", "Run tests");
        assert_eq!(auto.name, "daily-test");
        assert_eq!(auto.schedule, "0 9 * * *");
        assert_eq!(auto.status, "active");
    }

    #[test]
    fn test_list_automations() {
        let mut mgr = AutomationManager::new();
        mgr.create("a1", "* * * * *", "task1");
        mgr.create("a2", "*/5 * * * *", "task2");
        assert_eq!(mgr.list().len(), 2);
    }

    #[test]
    fn test_get_by_id() {
        let mut mgr = AutomationManager::new();
        let auto = mgr.create("test", "0 0 * * *", "test");
        assert!(mgr.get(&auto.id).is_some());
    }

    #[test]
    fn test_get_by_name() {
        let mut mgr = AutomationManager::new();
        mgr.create("my-auto", "0 0 * * *", "test");
        assert!(mgr.get("my-auto").is_some());
    }

    #[test]
    fn test_pause_resume() {
        let mut mgr = AutomationManager::new();
        let auto = mgr.create("test", "* * * * *", "test");
        assert!(mgr.set_status(&auto.id, AutomationStatus::Paused));
        assert_eq!(mgr.get(&auto.id).unwrap().status, "paused");
        assert!(mgr.set_status(&auto.id, AutomationStatus::Active));
    }

    #[test]
    fn test_delete() {
        let mut mgr = AutomationManager::new();
        let auto = mgr.create("test", "* * * * *", "test");
        assert_eq!(mgr.list().len(), 1);
        assert!(mgr.delete(&auto.id));
        assert_eq!(mgr.list().len(), 0);
    }

    #[test]
    fn test_run_now() {
        let mut mgr = AutomationManager::new();
        let auto = mgr.create("test", "* * * * *", "hello");
        let result = mgr.run_now(&auto.id);
        assert!(result.is_some());
        assert!(result.unwrap().contains("hello"));
        assert!(mgr.get(&auto.id).unwrap().last_run.is_some());
    }

    #[test]
    fn test_format_empty() {
        let mgr = AutomationManager::new();
        assert!(mgr.format_list().contains("No automations"));
    }

    #[test]
    fn test_update() {
        let mut mgr = AutomationManager::new();
        let auto = mgr.create("test", "0 0 * * *", "old");
        assert!(mgr.update(&auto.id, Some("renamed"), Some("*/5 * * * *"), Some("new prompt"), None, None));
        let updated = mgr.get(&auto.id).unwrap();
        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.prompt, "new prompt");
    }

    #[test]
    fn test_status_transitions() {
        assert_eq!(AutomationStatus::Active.as_str(), "active");
        assert_eq!(AutomationStatus::Paused.as_str(), "paused");
        assert_eq!(AutomationStatus::from_str("paused"), AutomationStatus::Paused);
        assert_eq!(AutomationStatus::from_str("unknown"), AutomationStatus::Active);
    }
}

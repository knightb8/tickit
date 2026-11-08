use tempfile::tempdir;
use tickit::app::state::{AppState, Mode};
use tickit::config::Config;
use tickit::db::Database;
use tickit::models::{Priority, Task};

#[test]
fn test_external_change_detected_on_insert() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite");

    // Session 1 (TUI state)
    let db1 = Database::open_path(&db_path).unwrap();
    let config = Config::default();
    let mut state = AppState::new(config, db1).unwrap();
    let inbox_id = state.lists.iter().find(|l| l.is_inbox).unwrap().id;

    assert_eq!(state.tasks.len(), 0);
    assert_eq!(state.check_external_db_changes().unwrap(), false);

    // Session 2 (e.g. CLI command `tickit add "External task"`)
    let db2 = Database::open_path(&db_path).unwrap();
    let task = Task::new("External task from CLI", inbox_id);
    db2.insert_task(&task).unwrap();

    // Session 1 scans DB
    assert_eq!(state.check_external_db_changes().unwrap(), true);
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].title, "External task from CLI");

    // Next scan without any DB changes returns false
    assert_eq!(state.check_external_db_changes().unwrap(), false);
}

#[test]
fn test_external_change_detected_on_update() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite");

    let db1 = Database::open_path(&db_path).unwrap();
    let mut state = AppState::new(Config::default(), db1).unwrap();
    let inbox_id = state.lists.iter().find(|l| l.is_inbox).unwrap().id;

    let db2 = Database::open_path(&db_path).unwrap();
    let mut task = Task::new("Task 1", inbox_id);
    db2.insert_task(&task).unwrap();

    state.check_external_db_changes().unwrap();
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].completed, false);

    // External session updates task to completed
    task.completed = true;
    task.completed_at = Some(chrono::Utc::now());
    db2.update_task(&task).unwrap();

    assert_eq!(state.check_external_db_changes().unwrap(), true);
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].completed, true);
}

#[test]
fn test_editor_mode_defers_reload() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite");

    let db1 = Database::open_path(&db_path).unwrap();
    let mut state = AppState::new(Config::default(), db1).unwrap();
    let inbox_id = state.lists.iter().find(|l| l.is_inbox).unwrap().id;

    let db2 = Database::open_path(&db_path).unwrap();

    // User is in AddTask modal
    state.start_add_task();
    assert_eq!(state.mode, Mode::AddTask);

    // External change occurs while typing
    let task = Task::new("Background task", inbox_id);
    db2.insert_task(&task).unwrap();

    // Scan should NOT reload while in AddTask mode
    assert_eq!(state.check_external_db_changes().unwrap(), false);
    assert_eq!(state.tasks.len(), 0);

    // User exits modal back to Normal
    state.mode = Mode::Normal;

    // Next scan now reloads cleanly
    assert_eq!(state.check_external_db_changes().unwrap(), true);
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].title, "Background task");
}

#[test]
fn test_selected_task_preserved_across_external_reload() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite");

    let db1 = Database::open_path(&db_path).unwrap();
    let mut state = AppState::new(Config::default(), db1).unwrap();
    let inbox_id = state.lists.iter().find(|l| l.is_inbox).unwrap().id;

    let db2 = Database::open_path(&db_path).unwrap();
    let t1 = Task::new("Task 1 (Low)", inbox_id);
    let mut t2 = Task::new("Task 2 (Med)", inbox_id);
    t2.priority = Priority::Medium;
    db2.insert_task(&t1).unwrap();
    db2.insert_task(&t2).unwrap();

    state.refresh_data().unwrap();
    assert_eq!(state.tasks.len(), 2);

    // Select Task 1
    state.task_index = state.tasks.iter().position(|t| t.id == t1.id).unwrap();
    assert_eq!(state.selected_task().unwrap().id, t1.id);

    // External session inserts Task 3 with Urgent priority (which sorts to the top)
    let mut t3 = Task::new("Task 3 (Urgent)", inbox_id);
    t3.priority = Priority::Urgent;
    db2.insert_task(&t3).unwrap();

    // Scan detects change and reloads
    assert_eq!(state.check_external_db_changes().unwrap(), true);
    assert_eq!(state.tasks.len(), 3);

    // Cursor remains on Task 1
    assert_eq!(state.selected_task().unwrap().id, t1.id);
}

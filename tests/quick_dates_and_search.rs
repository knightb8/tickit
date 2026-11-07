use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::tempdir;
use tickit::app::events::handle_key;
use tickit::app::state::{AppState, EditorField, Mode, QuickDatePreset};
use tickit::config::Config;
use tickit::db::Database;
use tickit::models::{Tag, Task};

fn fixed_datetime(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
        .unwrap()
}

fn create_test_app() -> (AppState, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite");
    let db = Database::open_path(&db_path).unwrap();
    let config = Config::default();
    let mut state = AppState::new(config, db).unwrap();
    state.refresh_data().unwrap();
    (state, dir)
}

#[test]
fn test_scenario_a1_preset_date_calculations() {
    // Wednesday, Nov 4, 2026
    let wednesday = fixed_datetime(2026, 11, 4, 12, 0, 0);
    assert_eq!(
        QuickDatePreset::Today.to_date(wednesday),
        NaiveDate::from_ymd_opt(2026, 11, 4).unwrap()
    );
    assert_eq!(
        QuickDatePreset::Tomorrow.to_date(wednesday),
        NaiveDate::from_ymd_opt(2026, 11, 5).unwrap()
    );
    // Upcoming Saturday
    assert_eq!(
        QuickDatePreset::Weekend.to_date(wednesday),
        NaiveDate::from_ymd_opt(2026, 11, 7).unwrap()
    );
    // Next Monday
    assert_eq!(
        QuickDatePreset::NextWeek.to_date(wednesday),
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    );
    // +14 days
    assert_eq!(
        QuickDatePreset::TwoWeeks.to_date(wednesday),
        NaiveDate::from_ymd_opt(2026, 11, 18).unwrap()
    );

    // Saturday, Nov 7, 2026: weekend should return following Saturday (Nov 14)
    let saturday = fixed_datetime(2026, 11, 7, 10, 0, 0);
    assert_eq!(
        QuickDatePreset::Weekend.to_date(saturday),
        NaiveDate::from_ymd_opt(2026, 11, 14).unwrap()
    );
    assert_eq!(
        QuickDatePreset::NextWeek.to_date(saturday),
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    );

    // Sunday, Nov 8, 2026: weekend should return following Saturday (Nov 14), next week returns Nov 9
    let sunday = fixed_datetime(2026, 11, 8, 14, 0, 0);
    assert_eq!(
        QuickDatePreset::Weekend.to_date(sunday),
        NaiveDate::from_ymd_opt(2026, 11, 14).unwrap()
    );
    assert_eq!(
        QuickDatePreset::NextWeek.to_date(sunday),
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    );
}

#[test]
fn test_scenario_a2_natural_language_date_parsing() {
    let now = fixed_datetime(2026, 11, 4, 10, 0, 0); // Wednesday

    // "today", "tod", "t"
    let expected_today = fixed_datetime(2026, 11, 4, 23, 59, 59);
    assert_eq!(
        AppState::parse_due_date_with_ref("today", now),
        Some(expected_today)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("TOD", now),
        Some(expected_today)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("t", now),
        Some(expected_today)
    );

    // "tomorrow", "tom", "m"
    let expected_tomorrow = fixed_datetime(2026, 11, 5, 23, 59, 59);
    assert_eq!(
        AppState::parse_due_date_with_ref("tomorrow", now),
        Some(expected_tomorrow)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("tom", now),
        Some(expected_tomorrow)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("m", now),
        Some(expected_tomorrow)
    );

    // "weekend", "sat"
    let expected_weekend = fixed_datetime(2026, 11, 7, 23, 59, 59);
    assert_eq!(
        AppState::parse_due_date_with_ref("weekend", now),
        Some(expected_weekend)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("sat", now),
        Some(expected_weekend)
    );

    // "next week", "week", "mon", "1w"
    let expected_next_week = fixed_datetime(2026, 11, 9, 23, 59, 59);
    assert_eq!(
        AppState::parse_due_date_with_ref("next week", now),
        Some(expected_next_week)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("week", now),
        Some(expected_next_week)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("mon", now),
        Some(expected_next_week)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("1w", now),
        Some(expected_next_week)
    );

    // "two weeks", "2w", "2 weeks"
    let expected_two_weeks = fixed_datetime(2026, 11, 18, 23, 59, 59);
    assert_eq!(
        AppState::parse_due_date_with_ref("two weeks", now),
        Some(expected_two_weeks)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("2w", now),
        Some(expected_two_weeks)
    );
    assert_eq!(
        AppState::parse_due_date_with_ref("2 weeks", now),
        Some(expected_two_weeks)
    );

    // ISO format
    let expected_iso = fixed_datetime(2026, 12, 25, 23, 59, 59);
    assert_eq!(
        AppState::parse_due_date_with_ref("2026-12-25", now),
        Some(expected_iso)
    );

    // Empty or invalid format
    assert_eq!(AppState::parse_due_date_with_ref("", now), None);
    assert_eq!(AppState::parse_due_date_with_ref("not a date", now), None);
}

#[test]
fn test_scenario_a3_keyboard_interaction_in_due_date_field() {
    let (mut state, _dir) = create_test_app();

    state.start_add_task();
    state.editor_field = EditorField::DueDate;
    state.input_buffer.clear();
    state.cursor_pos = 0;

    let now = Utc::now();

    // Alt+1 / F1 => Today
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('1'), KeyModifiers::ALT),
    );
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::Today.to_date_string(now)
    );

    // Alt+2 / F2 => Tomorrow
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('2'), KeyModifiers::ALT),
    );
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::Tomorrow.to_date_string(now)
    );

    // F3 => Weekend
    handle_key(&mut state, KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::Weekend.to_date_string(now)
    );

    // Alt+4 => Next Week
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('4'), KeyModifiers::ALT),
    );
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::NextWeek.to_date_string(now)
    );

    // Alt+5 => Two Weeks
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('5'), KeyModifiers::ALT),
    );
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::TwoWeeks.to_date_string(now)
    );

    // Cycling with Down / j
    handle_key(&mut state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::Today.to_date_string(now)
    );

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
    );
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::Tomorrow.to_date_string(now)
    );

    // Cycling with Up / k
    handle_key(&mut state, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::Today.to_date_string(now)
    );

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
    );
    assert_eq!(
        state.input_buffer,
        QuickDatePreset::TwoWeeks.to_date_string(now)
    );
}

#[test]
fn test_scenario_b1_b2_b3_search_filtering_and_lifecycle() {
    let (mut state, _dir) = create_test_app();
    let inbox_id = state.lists.iter().find(|l| l.is_inbox).unwrap().id;

    // Create a tag
    let camera_tag = Tag::new("camera-hardware");
    state.db.insert_tag(&camera_tag).unwrap();
    state.refresh_data().unwrap();

    // Insert 4 tasks
    let t1 = Task::new("Fix camera shutter glitch", inbox_id);
    state.db.insert_task(&t1).unwrap();

    let mut t2 = Task::new("Refactor authentication flow", inbox_id);
    t2.description = Some("Need to verify with camera token".to_string());
    state.db.insert_task(&t2).unwrap();

    let mut t3 = Task::new("Update system documentation", inbox_id);
    t3.tag_ids = vec![camera_tag.id];
    state.db.insert_task(&t3).unwrap();

    let t4 = Task::new("Grocery shopping for dinner", inbox_id);
    state.db.insert_task(&t4).unwrap();

    state.refresh_data().unwrap();
    assert_eq!(state.tasks.len(), 4);

    // B1: Press '/' to activate Search Mode
    assert_eq!(state.mode, Mode::Normal);
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
    );
    assert_eq!(state.mode, Mode::Search);

    // B2: Live filtering as user types "camera"
    for c in "camera".chars() {
        handle_key(
            &mut state,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    assert_eq!(state.search_query, "camera");
    assert_eq!(state.tasks.len(), 3);
    assert!(state.tasks.iter().any(|t| t.title.contains("Fix camera")));
    assert!(
        state
            .tasks
            .iter()
            .any(|t| t.title.contains("authentication"))
    );
    assert!(
        state
            .tasks
            .iter()
            .any(|t| t.title.contains("documentation"))
    );
    assert!(!state.tasks.iter().any(|t| t.title.contains("Grocery")));
    assert_eq!(state.task_index, 0);

    // Backspace test
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
    );
    assert_eq!(state.search_query, "camer");
    assert_eq!(state.tasks.len(), 3);

    // B3: Press Enter to exit search mode while keeping filter active
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert_eq!(state.mode, Mode::Normal);
    assert_eq!(state.search_query, "camer");
    assert_eq!(state.tasks.len(), 3);

    // In Normal mode, pressing Esc clears active search and restores all tasks
    handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(state.search_query, "");
    assert_eq!(state.tasks.len(), 4);

    // Search mode Esc clearing
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
    );
    for c in "dinner".chars() {
        handle_key(
            &mut state,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    assert_eq!(state.tasks.len(), 1); // Grocery shopping for dinner
    // First Esc clears query
    handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(state.search_query, "");
    assert_eq!(state.tasks.len(), 4);
    assert_eq!(state.mode, Mode::Search);
    // Second Esc exits search mode
    handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(state.mode, Mode::Normal);
}

#[test]
fn test_scenario_b4_search_shortcuts_ctrl_u() {
    let (mut state, _dir) = create_test_app();
    let inbox_id = state.lists.iter().find(|l| l.is_inbox).unwrap().id;

    let t1 = Task::new("Buy milk", inbox_id);
    let t2 = Task::new("Buy cereal", inbox_id);
    state.db.insert_task(&t1).unwrap();
    state.db.insert_task(&t2).unwrap();
    state.refresh_data().unwrap();

    // Start search and type "cereal"
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
    );
    for c in "cereal".chars() {
        handle_key(
            &mut state,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    assert_eq!(state.tasks.len(), 1);

    // Confirm search with Enter to go back to Normal mode
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert_eq!(state.mode, Mode::Normal);
    assert_eq!(state.search_query, "cereal");
    assert_eq!(state.tasks.len(), 1);

    // Press Ctrl+u in Normal mode -> clears search query and restores all tasks
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_eq!(state.search_query, "");
    assert_eq!(state.tasks.len(), 2);
}

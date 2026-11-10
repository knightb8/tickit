use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::tempdir;
use tickit::app::events::handle_key;
use tickit::app::state::{AppState, Focus, Mode, SettingsCategory, View};
use tickit::config::Config;
use tickit::db::Database;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(100);

fn create_test_state() -> (AppState, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = dir.path().join(format!("test_settings_{}.sqlite", id));
    let db = Database::open_path(&path).unwrap();
    let config = Config::default();
    let mut state = AppState::new(config, db).unwrap();
    state.refresh_data().unwrap();
    (state, dir)
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shift_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

#[test]
fn test_tab_in_normal_mode_toggles_focus_between_sidebar_and_main() {
    let (mut state, _dir) = create_test_state();
    assert_eq!(state.focus, Focus::Main);

    // Tab switches to Sidebar
    handle_key(&mut state, key(KeyCode::Tab));
    assert_eq!(state.focus, Focus::Sidebar);

    // Tab switches back to Main
    handle_key(&mut state, key(KeyCode::Tab));
    assert_eq!(state.focus, Focus::Main);

    // BackTab (Shift+Tab) switches to Sidebar
    handle_key(&mut state, shift_key(KeyCode::BackTab));
    assert_eq!(state.focus, Focus::Sidebar);
}

#[test]
fn test_keys_1_2_3_do_not_switch_views() {
    let (mut state, _dir) = create_test_state();
    assert_eq!(state.view, View::Tasks);

    // Pressing '1', '2', '3' should not change view from Tasks
    handle_key(&mut state, key(KeyCode::Char('1')));
    assert_eq!(state.view, View::Tasks);

    handle_key(&mut state, key(KeyCode::Char('2')));
    assert_eq!(state.view, View::Tasks);

    handle_key(&mut state, key(KeyCode::Char('3')));
    assert_eq!(state.view, View::Tasks);
}

#[test]
fn test_settings_dialog_category_cycling_and_actions() {
    let (mut state, _dir) = create_test_state();

    // Open settings with 's'
    handle_key(&mut state, key(KeyCode::Char('s')));
    assert_eq!(state.mode, Mode::Settings);
    assert_eq!(state.settings_category, SettingsCategory::Appearance);
    assert_eq!(state.settings_index, 0);

    // Tab cycles categories forward
    handle_key(&mut state, key(KeyCode::Tab));
    assert_eq!(state.settings_category, SettingsCategory::Behavior);

    handle_key(&mut state, key(KeyCode::Tab));
    assert_eq!(state.settings_category, SettingsCategory::Lists);

    handle_key(&mut state, key(KeyCode::Tab));
    assert_eq!(state.settings_category, SettingsCategory::Tags);

    handle_key(&mut state, key(KeyCode::Tab));
    assert_eq!(state.settings_category, SettingsCategory::Sync);

    handle_key(&mut state, key(KeyCode::Tab));
    assert_eq!(state.settings_category, SettingsCategory::About);

    handle_key(&mut state, key(KeyCode::Tab));
    assert_eq!(state.settings_category, SettingsCategory::Appearance);

    // Shift+Tab or '[' cycles backward
    handle_key(&mut state, key(KeyCode::Char('[')));
    assert_eq!(state.settings_category, SettingsCategory::About);

    handle_key(&mut state, key(KeyCode::Char('[')));
    assert_eq!(state.settings_category, SettingsCategory::Sync);

    // Esc closes settings
    handle_key(&mut state, key(KeyCode::Esc));
    assert_eq!(state.mode, Mode::Normal);
}

#[test]
fn test_settings_appearance_theme_stepping() {
    let (mut state, _dir) = create_test_state();

    // Open settings
    handle_key(&mut state, key(KeyCode::Char('s')));
    assert_eq!(state.settings_category, SettingsCategory::Appearance);
    let initial_theme = state.theme.name().to_string();

    // Right arrow steps to next theme
    handle_key(&mut state, key(KeyCode::Right));
    let next_theme = state.theme.name().to_string();
    assert_ne!(initial_theme, next_theme);

    // Left arrow steps back
    handle_key(&mut state, key(KeyCode::Left));
    assert_eq!(state.theme.name(), initial_theme);
}

#[test]
fn test_settings_behavior_toggles() {
    let (mut state, _dir) = create_test_state();

    handle_key(&mut state, key(KeyCode::Char('s')));
    handle_key(&mut state, key(KeyCode::Tab)); // To Behavior
    assert_eq!(state.settings_category, SettingsCategory::Behavior);

    let initial_show_completed = state.config.show_completed;
    // Enter / Space toggles Show Completed
    handle_key(&mut state, key(KeyCode::Enter));
    assert_eq!(state.config.show_completed, !initial_show_completed);

    // Down to EnterCompletes
    handle_key(&mut state, key(KeyCode::Down));
    assert_eq!(state.settings_index, 1);
    let initial_enter_completes = state.config.enter_completes;
    handle_key(&mut state, key(KeyCode::Char(' ')));
    assert_eq!(state.config.enter_completes, !initial_enter_completes);
}

#[test]
fn test_settings_lists_management_workflow() {
    let (mut state, _dir) = create_test_state();

    // Open settings and go to Lists
    handle_key(&mut state, key(KeyCode::Char('s')));
    handle_key(&mut state, key(KeyCode::Tab)); // Behavior
    handle_key(&mut state, key(KeyCode::Tab)); // Lists
    assert_eq!(state.settings_category, SettingsCategory::Lists);

    let initial_lists_count = state.lists.len();

    // Press 'a' to add a new list
    handle_key(&mut state, key(KeyCode::Char('a')));
    assert_eq!(state.mode, Mode::AddList);
    assert!(state.return_to_settings);

    // Type "Projects" and save with Enter
    for c in "Projects".chars() {
        handle_key(&mut state, key(KeyCode::Char(c)));
    }
    handle_key(&mut state, key(KeyCode::Enter));

    // Should return to Settings dialog!
    assert_eq!(state.mode, Mode::Settings);
    assert_eq!(state.settings_category, SettingsCategory::Lists);
    assert_eq!(state.lists.len(), initial_lists_count + 1);
    assert!(state.lists.iter().any(|l| l.name == "Projects"));

    // Navigate to the newly created list
    state.list_index = state
        .lists
        .iter()
        .position(|l| l.name == "Projects")
        .unwrap();

    // Press 'e' to edit list name
    handle_key(&mut state, key(KeyCode::Char('e')));
    assert_eq!(state.mode, Mode::EditList);
    assert!(state.return_to_settings);

    // Cancel edit with Esc should return to Settings
    handle_key(&mut state, key(KeyCode::Esc));
    assert_eq!(state.mode, Mode::Settings);
    assert_eq!(state.settings_category, SettingsCategory::Lists);

    // Press 'd' to delete list
    handle_key(&mut state, key(KeyCode::Char('d')));
    assert_eq!(state.mode, Mode::Confirm);
    assert!(state.return_to_settings);

    // Confirm deletion with 'y'
    handle_key(&mut state, key(KeyCode::Char('y')));
    assert_eq!(state.mode, Mode::Settings);
    assert_eq!(state.settings_category, SettingsCategory::Lists);
    assert_eq!(state.lists.len(), initial_lists_count);
}

#[test]
fn test_settings_tags_management_workflow() {
    let (mut state, _dir) = create_test_state();

    // Open settings and go to Tags
    handle_key(&mut state, key(KeyCode::Char('s')));
    handle_key(&mut state, key(KeyCode::Tab)); // Behavior
    handle_key(&mut state, key(KeyCode::Tab)); // Lists
    handle_key(&mut state, key(KeyCode::Tab)); // Tags
    assert_eq!(state.settings_category, SettingsCategory::Tags);

    let initial_tags_count = state.tags.len();

    // Press 'a' to add a new tag
    handle_key(&mut state, key(KeyCode::Char('a')));
    assert_eq!(state.mode, Mode::AddTag);
    assert!(state.return_to_settings);

    // Type "urgent" and save with Enter
    for c in "urgent".chars() {
        handle_key(&mut state, key(KeyCode::Char(c)));
    }
    handle_key(&mut state, key(KeyCode::Enter));

    // Returns to Settings dialog in Tags category
    assert_eq!(state.mode, Mode::Settings);
    assert_eq!(state.settings_category, SettingsCategory::Tags);
    assert_eq!(state.tags.len(), initial_tags_count + 1);
    assert!(state.tags.iter().any(|t| t.name == "urgent"));
}

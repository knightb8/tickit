use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::tempdir;
use tickit::app::events::handle_key;
use tickit::app::state::{AppState, EditorField, Mode};
use tickit::config::Config;
use tickit::db::Database;

fn create_test_state() -> (AppState, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite");
    let db = Database::open_path(&db_path).unwrap();
    let config = Config::default();
    let mut state = AppState::new(config, db).unwrap();
    state.refresh_data().unwrap();
    (state, dir)
}

#[test]
fn test_exact_repro_umlaut_space_in_input_buffer() {
    let (mut state, _dir) = create_test_state();

    // Exact user crash scenario: type 'ü' then ' ' (space)
    state.input_insert_char('ü');
    assert_eq!(state.input_buffer, "ü");
    assert_eq!(state.cursor_pos, 2); // 'ü' is 2 bytes in UTF-8
    assert!(state.input_buffer.is_char_boundary(state.cursor_pos));

    // Next character: space - previously panicked with assertion failed: self.is_char_boundary(idx)
    state.input_insert_char(' ');
    assert_eq!(state.input_buffer, "ü ");
    assert_eq!(state.cursor_pos, 3);
    assert!(state.input_buffer.is_char_boundary(state.cursor_pos));

    // Type additional text: "überraschung"
    for c in "berraschung".chars() {
        state.input_insert_char(c);
        assert!(state.input_buffer.is_char_boundary(state.cursor_pos));
    }
    assert_eq!(state.input_buffer, "ü berraschung");
}

#[test]
fn test_unicode_navigation_and_editing() {
    let (mut state, _dir) = create_test_state();

    // Type "Büro"
    for c in "Büro".chars() {
        state.input_insert_char(c);
    }
    assert_eq!(state.input_buffer, "Büro");
    // 'B' (1) + 'ü' (2) + 'r' (1) + 'o' (1) = 5 bytes
    assert_eq!(state.cursor_pos, 5);

    // Move left twice: past 'o', then past 'r'
    state.input_move_left(); // after 'r'
    assert_eq!(state.cursor_pos, 4);
    assert!(state.input_buffer.is_char_boundary(state.cursor_pos));

    state.input_move_left(); // after 'ü'
    assert_eq!(state.cursor_pos, 3);
    assert!(state.input_buffer.is_char_boundary(state.cursor_pos));

    // Move left past 'ü' (2 bytes!)
    state.input_move_left(); // after 'B'
    assert_eq!(state.cursor_pos, 1);
    assert!(state.input_buffer.is_char_boundary(state.cursor_pos));

    // Insert '-' between 'B' and 'ü'
    state.input_insert_char('-');
    assert_eq!(state.input_buffer, "B-üro");
    assert_eq!(state.cursor_pos, 2);

    // Move right past 'ü'
    state.input_move_right();
    assert_eq!(state.cursor_pos, 4); // after 'ü'

    // Backspace: should delete 'ü'
    state.input_backspace();
    assert_eq!(state.input_buffer, "B-ro");
    assert_eq!(state.cursor_pos, 2);

    // Delete at cursor: should delete 'r'
    state.input_delete();
    assert_eq!(state.input_buffer, "B-o");
}

#[test]
fn test_event_handling_in_task_editor_description() {
    let (mut state, _dir) = create_test_state();

    // Start adding a task and switch to Description field
    state.start_add_task();
    assert_eq!(state.mode, Mode::AddTask);
    state.next_editor_field(); // Title -> Description
    assert_eq!(state.editor_field, EditorField::Description);

    // Simulate key events: 'ü' then ' ' then 't' then 'e' then 's' then 't'
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('ü'), KeyModifiers::NONE),
    );
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
    );
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
    );
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
    );

    assert_eq!(state.input_buffer, "ü test");
    assert!(state.input_buffer.is_char_boundary(state.cursor_pos));

    // Simulate Shift+Enter for newline
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT),
    );
    assert_eq!(state.input_buffer, "ü test\n");

    // Add second line with umlauts: "Äpfel & Öfen"
    for c in "Äpfel & Öfen".chars() {
        handle_key(
            &mut state,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    assert_eq!(state.input_buffer, "ü test\nÄpfel & Öfen");

    // Test cursor display offset for multi-line description
    let (col, row) = state.input_cursor_display_offset();
    assert_eq!(row, 1);
    assert_eq!(col, 12); // "Äpfel & Öfen" is 12 characters wide
}

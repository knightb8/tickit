//! UI rendering for the TUI

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use super::state::{
    AppState, DueFilter, EditorField, Focus, Mode, QuickDatePreset, SettingsCategory, SettingsItem,
    View,
};
use crate::theme::Theme;

/// ASCII art logo for Tickit (used in help screen)
#[allow(dead_code)]
const LOGO: &str = r#"
████████╗██╗ ██████╗██╗  ██╗██╗████████╗
╚══██╔══╝██║██╔════╝██║ ██╔╝██║╚══██╔══╝
   ██║   ██║██║     █████╔╝ ██║   ██║
   ██║   ██║██║     ██╔═██╗ ██║   ██║
   ██║   ██║╚██████╗██║  ██╗██║   ██║
   ╚═╝   ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝   ╚═╝
"#;

/// Tickit icon
const ICON: &str = "✓";

/// Render the entire UI
pub fn render(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();

    // Set background
    let area = frame.area();
    let bg_block = Block::default().style(Style::default().bg(colors.bg));
    frame.render_widget(bg_block, area);

    // Check if we need to show update banner
    let has_update = state.update_available.is_some();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_update {
            vec![
                Constraint::Length(3), // Update banner
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Main content
                Constraint::Length(1), // Status bar
            ]
        } else {
            vec![
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Main content
                Constraint::Length(1), // Status bar
            ]
        })
        .split(area);

    let (tabs_area, main_area, status_area) = if has_update {
        render_update_banner(frame, state, chunks[0]);
        (chunks[1], chunks[2], chunks[3])
    } else {
        (chunks[0], chunks[1], chunks[2])
    };

    render_tabs(frame, state, tabs_area);
    render_main(frame, state, main_area);
    render_status_bar(frame, state, status_area);

    // Render popups/dialogs
    if state.show_help || state.mode == Mode::Help {
        render_help_popup(frame, state);
    }

    if state.mode == Mode::ThemePicker {
        render_theme_picker(frame, state);
    }

    if state.mode == Mode::Settings || state.mode == Mode::SettingsInput {
        render_settings_dialog(frame, state);
    }

    if state.mode == Mode::Confirm {
        render_confirm_dialog(frame, state);
    }

    if state.mode == Mode::UpdateConfirm {
        render_update_confirm_dialog(frame, state);
    }

    if state.mode == Mode::Updating {
        render_updating_overlay(frame, state);
    }

    if matches!(state.mode, Mode::AddTask | Mode::EditTask) {
        render_task_editor(frame, state);
    }

    if matches!(state.mode, Mode::AddList | Mode::EditList) {
        render_simple_editor(frame, state, "List");
    }

    if matches!(state.mode, Mode::AddTag | Mode::EditTag) {
        render_simple_editor(frame, state, "Tag");
    }

    if state.mode == Mode::About {
        render_about_dialog(frame, state);
    }
}

const LEFT_COLUMN_PERCENT: u16 = 36;

/// Render the unified top bar: Branding on left, Due Filter on right
fn render_tabs(frame: &mut Frame, state: &AppState, area: Rect) {
    if area.width >= 75 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(LEFT_COLUMN_PERCENT),
                Constraint::Percentage(100 - LEFT_COLUMN_PERCENT),
            ])
            .split(area);

        render_header_branding(frame, state, chunks[0]);
        render_due_filter_header(frame, state, chunks[1]);
    } else {
        render_due_filter_header(frame, state, area);
    }
}

/// Render the header branding block on the left
fn render_header_branding(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let mut spans = vec![Span::styled(
        format!(" {} Tickit ", ICON),
        colors.logo_style_primary(),
    )];

    if area.width >= 24 {
        spans.push(Span::styled("[s: settings]", colors.key_hint()));
    }

    let p = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.block()),
        )
        .alignment(Alignment::Left);

    frame.render_widget(p, area);
}

fn due_filter_pill_label(filter: DueFilter, count: usize, compact: bool) -> String {
    if compact {
        match filter {
            DueFilter::All => format!("{} All: {}", filter.icon(), count),
            DueFilter::Overdue => format!("{} Due: {}", filter.icon(), count),
            DueFilter::Today => format!("{} Today: {}", filter.icon(), count),
            DueFilter::ThisWeek => format!("{} Wk: {}", filter.icon(), count),
            DueFilter::NoDueDate => format!("{} None: {}", filter.icon(), count),
        }
    } else {
        match filter {
            DueFilter::All => format!("{} All ({})", filter.icon(), count),
            DueFilter::Overdue => format!("{} Overdue ({})", filter.icon(), count),
            DueFilter::Today => format!("{} Today ({})", filter.icon(), count),
            DueFilter::ThisWeek => format!("{} This Week ({})", filter.icon(), count),
            DueFilter::NoDueDate => format!("{} No Date ({})", filter.icon(), count),
        }
    }
}

/// Render the due-ness filter in the unified top bar
fn render_due_filter_header(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();
    let compact = area.width < 90;

    let mut spans = Vec::new();
    spans.push(Span::raw(" "));

    for (idx, filter) in DueFilter::all().iter().enumerate() {
        let is_selected = *filter == state.due_filter;
        let count = state.due_filter_counts.get(idx).copied().unwrap_or(0);
        let label = due_filter_pill_label(*filter, count, compact);

        let filter_style = if is_selected {
            match filter {
                DueFilter::All => colors.selected().add_modifier(Modifier::BOLD),
                DueFilter::Overdue => Style::default()
                    .fg(colors.bg)
                    .bg(colors.error)
                    .add_modifier(Modifier::BOLD),
                DueFilter::Today => Style::default()
                    .fg(colors.bg)
                    .bg(colors.warning)
                    .add_modifier(Modifier::BOLD),
                DueFilter::ThisWeek => Style::default()
                    .fg(colors.bg)
                    .bg(colors.info)
                    .add_modifier(Modifier::BOLD),
                DueFilter::NoDueDate => Style::default()
                    .fg(colors.bg)
                    .bg(colors.secondary)
                    .add_modifier(Modifier::BOLD),
            }
        } else {
            match filter {
                DueFilter::Overdue if count > 0 => colors.text_error(),
                DueFilter::Today if count > 0 => colors.text_warning(),
                DueFilter::ThisWeek if count > 0 => colors.text_info(),
                _ => colors.text_muted(),
            }
        };

        if is_selected {
            spans.push(Span::styled(format!(" [ {} ] ", label), filter_style));
        } else {
            spans.push(Span::styled(format!("  {}  ", label), filter_style));
        }
        spans.push(Span::raw(" "));
    }

    let header_block = Block::default()
        .title(" Due Filter [f: cycle / F: prev] ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if state.due_filter != DueFilter::All {
            colors.block_focus()
        } else {
            colors.block()
        });

    let p = Paragraph::new(Line::from(spans))
        .block(header_block)
        .alignment(Alignment::Left);

    frame.render_widget(p, area);
}

/// Render the main content area
fn render_main(frame: &mut Frame, state: &AppState, area: Rect) {
    match state.view {
        View::Tasks => render_tasks_view(frame, state, area),
        View::Lists => render_lists_view(frame, state, area),
        View::Tags => render_tags_view(frame, state, area),
    }
}

/// Render the tasks view with sidebar and task details on left, task list on right
fn render_tasks_view(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(LEFT_COLUMN_PERCENT), // Left column (Details top + Lists bottom)
            Constraint::Percentage(100 - LEFT_COLUMN_PERCENT), // Right column (Tasks list)
        ])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(55), // Task Details (top of left column)
            Constraint::Percentage(45), // Lists sidebar (bottom of left column)
        ])
        .split(col_chunks[0]);

    // 1. Top of Left Column: Task Details
    render_task_detail_pane(frame, state, left_chunks[0]);

    // 2. Bottom of Left Column: Lists Sidebar
    let sidebar_focused = state.focus == Focus::Sidebar;
    let sidebar_style = if sidebar_focused {
        colors.block_focus()
    } else {
        colors.block()
    };

    let mut list_items: Vec<ListItem> = Vec::new();

    // Lists (no more "All" - Inbox shows all tasks)
    for (i, list) in state.lists.iter().enumerate() {
        let is_cursor = state.list_index == i;
        let is_active = if state.showing_recently_closed {
            false
        } else if let Some(sel_id) = state.selected_list_id {
            list.id == sel_id
        } else {
            list.is_inbox
        };
        let style = if is_cursor && sidebar_focused {
            colors.selected()
        } else if is_active {
            colors.text_accent().add_modifier(Modifier::BOLD)
        } else {
            colors.text()
        };
        let prefix = if is_active { "▸ " } else { "  " };

        // Inbox shows total task count (all tasks), other lists show their own count
        let count = if list.is_inbox {
            state
                .db
                .get_total_task_count(state.show_completed)
                .unwrap_or(0)
        } else {
            state
                .db
                .get_task_count(list.id, state.show_completed)
                .unwrap_or(0)
        };

        let clean_icon = list.icon.replace('\u{fe0f}', "");
        let max_name_len = (left_chunks[1].width.saturating_sub(11) as usize).max(8);
        let display_name = if list.name.chars().count() > max_name_len {
            let truncated: String = list
                .name
                .chars()
                .take(max_name_len.saturating_sub(1))
                .collect();
            format!("{}…", truncated)
        } else {
            list.name.clone()
        };

        list_items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{}{} ", prefix, clean_icon), style),
            Span::styled(display_name, style),
            Span::styled(format!(" ({})", count), colors.text_muted()),
        ])));
    }

    // Recently closed section in sidebar
    let rc_cursor = state.list_index == state.lists.len();
    let rc_active = state.showing_recently_closed;
    let rc_style = if rc_cursor && sidebar_focused {
        colors.selected()
    } else if rc_active {
        colors.text_accent().add_modifier(Modifier::BOLD)
    } else {
        colors.text()
    };
    let rc_prefix = if rc_active { "▸ " } else { "  " };
    let closed_count = state
        .db
        .get_recently_closed_tasks(100)
        .map(|t| t.len())
        .unwrap_or(0);
    list_items.push(ListItem::new(Line::from(vec![
        Span::styled(format!("{}✓ ", rc_prefix), rc_style),
        Span::styled("Recently Closed", rc_style),
        Span::styled(format!(" ({})", closed_count), colors.text_muted()),
    ])));

    let total_sidebar_items = list_items.len();
    let sidebar_height = (left_chunks[1].height.saturating_sub(2) as usize).max(1);
    let mut sb_offset = state.sidebar_scroll_offset.get();
    if state.list_index < sb_offset {
        sb_offset = state.list_index;
    } else if state.list_index >= sb_offset + sidebar_height {
        sb_offset = state.list_index.saturating_sub(sidebar_height - 1);
    }
    if total_sidebar_items > 0 && sb_offset >= total_sidebar_items {
        sb_offset = total_sidebar_items - 1;
    }
    state.sidebar_scroll_offset.set(sb_offset);

    let visible_sidebar_items: Vec<ListItem> = list_items
        .into_iter()
        .skip(sb_offset)
        .take(sidebar_height)
        .collect();

    let sidebar_title = if sidebar_focused {
        if total_sidebar_items > sidebar_height {
            let end_idx = (sb_offset + sidebar_height).min(total_sidebar_items);
            format!(
                " Lists [{}-{}/{} │ ↑/↓: filter, →/Enter: tasks] ",
                sb_offset + 1,
                end_idx,
                total_sidebar_items
            )
        } else {
            " Lists [↑/↓: filter │ →/Enter: tasks] ".to_string()
        }
    } else if total_sidebar_items > sidebar_height {
        let end_idx = (sb_offset + sidebar_height).min(total_sidebar_items);
        format!(
            " Lists [{}-{}/{} │ h/←: focus] ",
            sb_offset + 1,
            end_idx,
            total_sidebar_items
        )
    } else {
        " Lists [h/←: focus] ".to_string()
    };

    let sidebar = List::new(visible_sidebar_items).block(
        Block::default()
            .title(sidebar_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(sidebar_style),
    );

    frame.render_widget(sidebar, left_chunks[1]);

    // 3. Right Column: Task list
    let main_focused = state.focus == Focus::Main;
    let main_style = if main_focused {
        colors.block_focus()
    } else {
        colors.block()
    };

    let list_name = if state.showing_recently_closed {
        "Recently Closed".to_string()
    } else if let Some(list) = state.selected_list() {
        if list.is_inbox {
            "Inbox (All Tasks)".to_string()
        } else {
            list.name.clone()
        }
    } else {
        "All Tasks".to_string()
    };

    let list_rect = col_chunks[1];

    let visible_height = (list_rect.height.saturating_sub(2) as usize).max(1);
    let total_tasks = state.tasks.len();

    let mut offset = state.task_scroll_offset.get();
    if state.task_index < offset {
        offset = state.task_index;
    } else if state.task_index >= offset + visible_height {
        offset = state.task_index.saturating_sub(visible_height - 1);
    }
    if total_tasks > 0 && offset >= total_tasks {
        offset = total_tasks - 1;
    }
    state.task_scroll_offset.set(offset);

    let task_items: Vec<ListItem> = if state.tasks.is_empty() {
        let empty_msg = if state.due_filter != DueFilter::All {
            format!(
                "  No tasks matching due filter: {}",
                state.due_filter.name()
            )
        } else {
            "  No tasks in this list. Press 'n' to add one.".to_string()
        };
        vec![ListItem::new(Span::styled(empty_msg, colors.text_muted()))]
    } else {
        state
            .tasks
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible_height)
            .map(|(i, task)| {
                let selected = i == state.task_index;
                let base_style = if selected {
                    colors.selected()
                } else {
                    colors.text()
                };

                let checkbox = if task.completed { "☑" } else { "☐" };
                let checkbox_style = if task.completed {
                    colors.text_success()
                } else {
                    colors.text_muted()
                };

                let title_style = if task.completed {
                    base_style
                        .add_modifier(Modifier::CROSSED_OUT)
                        .fg(colors.fg_muted)
                } else {
                    base_style
                };

                let priority_style = colors.priority_style(task.priority);
                let priority_icon = task.priority.icon();

                let mut spans = vec![
                    Span::styled(format!(" {} ", checkbox), checkbox_style),
                    Span::styled(format!("{} ", priority_icon), priority_style),
                    Span::styled(&task.title, title_style),
                ];

                // Add due date indicator
                if let Some(due_date) = task.due_date {
                    let now = chrono::Utc::now();
                    let is_overdue = due_date < now && !task.completed;
                    let is_soon = due_date < now + chrono::Duration::days(2) && !is_overdue;

                    let due_style = if is_overdue {
                        colors.text_error()
                    } else if is_soon {
                        colors.text_warning()
                    } else {
                        colors.text_muted()
                    };

                    let due_str = due_date.format("%m/%d").to_string();
                    spans.push(Span::styled(format!(" 📅{}", due_str), due_style));
                }

                // Add URL indicator
                if task.url.is_some() {
                    spans.push(Span::styled(" 🔗", colors.text_info()));
                }

                // Add tag indicators
                if !task.tag_ids.is_empty() {
                    let tag_count = task.tag_ids.len();
                    spans.push(Span::styled(
                        format!(" [{}]", tag_count),
                        colors.text_secondary(),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect()
    };

    let show_status = if state.show_completed {
        ""
    } else {
        " (hiding completed)"
    };
    let search_tag = if !state.search_query.is_empty() {
        format!(" [Search: \"{}\"]", state.search_query)
    } else {
        String::new()
    };
    let title = if total_tasks > visible_height {
        let end_idx = (offset + visible_height).min(total_tasks);
        format!(
            " {}{} [{}-{}/{}] {} ",
            list_name,
            search_tag,
            offset + 1,
            end_idx,
            total_tasks,
            show_status
        )
    } else if total_tasks > 0 {
        format!(
            " {}{} [{}] {} ",
            list_name, search_tag, total_tasks, show_status
        )
    } else {
        format!(" {}{} {} ", list_name, search_tag, show_status)
    };

    let tasks_block = List::new(task_items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(main_style),
    );

    frame.render_widget(tasks_block, list_rect);
}

/// Render the task detail / preview pane
fn render_task_detail_pane(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();
    let detail_block = Block::default()
        .title(" Task Details ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.block());

    if let Some(task) = state.selected_task() {
        let inner = detail_block.inner(area);
        frame.render_widget(detail_block, area);

        let mut lines = Vec::new();

        // 1. Title
        lines.push(Line::from(vec![
            Span::styled(
                "Title: ",
                colors.text_secondary().add_modifier(Modifier::BOLD),
            ),
            Span::styled(&task.title, colors.text().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));

        // 2. Status & Priority
        let status_span = if task.completed {
            Span::styled("✓ Completed", colors.text_success())
        } else {
            Span::styled("☐ Pending", colors.text_warning())
        };
        let priority_style = colors.priority_style(task.priority);
        let priority_span = Span::styled(
            format!("{} {:?}", task.priority.icon(), task.priority),
            priority_style,
        );

        lines.push(Line::from(vec![
            Span::styled("Status: ", colors.text_secondary()),
            status_span,
            Span::styled("  |  Priority: ", colors.text_secondary()),
            priority_span,
        ]));

        // 3. Completed date (if completed)
        if let Some(completed_at) = task.completed_at {
            lines.push(Line::from(vec![
                Span::styled("Closed at: ", colors.text_secondary()),
                Span::styled(
                    completed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    colors.text_muted(),
                ),
            ]));
        }

        // 4. Due date & URL
        if let Some(due_date) = task.due_date {
            lines.push(Line::from(vec![
                Span::styled("Due date: ", colors.text_secondary()),
                Span::styled(due_date.format("%Y-%m-%d").to_string(), colors.text_info()),
            ]));
        }
        if let Some(url) = &task.url {
            lines.push(Line::from(vec![
                Span::styled("URL: ", colors.text_secondary()),
                Span::styled(url, colors.text_info()),
            ]));
        }

        // 5. Tags
        if !task.tag_ids.is_empty() {
            let mut tag_spans = vec![Span::styled("Tags: ", colors.text_secondary())];
            for (idx, tag_id) in task.tag_ids.iter().enumerate() {
                if idx > 0 {
                    tag_spans.push(Span::styled(" ", colors.text()));
                }
                if let Some(tag) = state.tags.iter().find(|t| t.id == *tag_id) {
                    let tag_color = Color::from_u32(
                        u32::from_str_radix(tag.color.trim_start_matches('#'), 16)
                            .unwrap_or(0x3498db),
                    );
                    tag_spans.push(Span::styled(
                        format!("[{}]", tag.name),
                        Style::default().fg(tag_color).add_modifier(Modifier::BOLD),
                    ));
                }
            }
            lines.push(Line::from(tag_spans));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "── Description ─────────────────────────",
            colors.text_muted(),
        )));

        // 6. Description
        if let Some(desc) = &task.description {
            if !desc.trim().is_empty() {
                for line in desc.lines() {
                    lines.push(Line::from(Span::styled(line, colors.text())));
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "(No description provided)",
                    colors.text_muted(),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "(No description provided)",
                colors.text_muted(),
            )));
        }

        let detail_content = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(detail_content, inner);
    } else {
        let empty_msg = Paragraph::new("No task selected")
            .style(colors.text_muted())
            .alignment(Alignment::Center)
            .block(detail_block);
        frame.render_widget(empty_msg, area);
    }
}

/// Render the lists view
fn render_lists_view(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let visible_height = (area.height.saturating_sub(2) as usize).max(1);
    let total_lists = state.lists.len();
    let mut offset = 0;
    if state.list_index >= visible_height {
        offset = state.list_index.saturating_sub(visible_height - 1);
    }
    if total_lists > 0 && offset >= total_lists {
        offset = total_lists - 1;
    }

    let list_items: Vec<ListItem> = state
        .lists
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_height)
        .map(|(i, list)| {
            let selected = i == state.list_index;
            let style = if selected {
                colors.selected()
            } else {
                colors.text()
            };

            let inbox_marker = if list.is_inbox { " (default)" } else { "" };

            ListItem::new(Line::from(vec![
                Span::styled(format!("  {} ", list.icon), style),
                Span::styled(&list.name, style),
                Span::styled(inbox_marker, colors.text_muted()),
            ]))
        })
        .collect();

    let title = if total_lists > visible_height {
        let end_idx = (offset + visible_height).min(total_lists);
        format!(" Lists [{}-{}/{}] ", offset + 1, end_idx, total_lists)
    } else {
        " Lists ".to_string()
    };

    let lists = List::new(list_items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(colors.block_focus()),
    );

    frame.render_widget(lists, area);
}

/// Render the tags view
fn render_tags_view(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let visible_height = (area.height.saturating_sub(2) as usize).max(1);
    let total_tags = state.tags.len();
    let mut offset = 0;
    if state.tag_index >= visible_height {
        offset = state.tag_index.saturating_sub(visible_height - 1);
    }
    if total_tags > 0 && offset >= total_tags {
        offset = total_tags - 1;
    }

    let tag_items: Vec<ListItem> = state
        .tags
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_height)
        .map(|(i, tag)| {
            let selected = i == state.tag_index;
            let style = if selected {
                colors.selected()
            } else {
                colors.text()
            };

            // Parse hex color for tag
            let tag_color = parse_hex_color(&tag.color).unwrap_or(colors.accent);

            ListItem::new(Line::from(vec![
                Span::styled("  ● ", Style::default().fg(tag_color)),
                Span::styled(&tag.name, style),
            ]))
        })
        .collect();

    let content = if state.tags.is_empty() {
        List::new(vec![ListItem::new(Span::styled(
            "  No tags yet. Press 'n' to add one.",
            colors.text_muted(),
        ))])
    } else {
        List::new(tag_items)
    };

    let title = if total_tags > visible_height {
        let end_idx = (offset + visible_height).min(total_tags);
        format!(" Tags [{}-{}/{}] ", offset + 1, end_idx, total_tags)
    } else {
        " Tags ".to_string()
    };

    let tags = content.block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(colors.block_focus()),
    );

    frame.render_widget(tags, area);
}

/// Render the status bar
fn render_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    // Calculate sync indicator width for layout
    let sync_width = if state.is_sync_enabled() { 16u16 } else { 0u16 };
    let content_width = area.width.saturating_sub(sync_width);

    // Split area into content and sync indicator
    let content_area = Rect {
        x: area.x,
        y: area.y,
        width: content_width,
        height: area.height,
    };
    let sync_area = Rect {
        x: area.x + content_width,
        y: area.y,
        width: sync_width,
        height: area.height,
    };

    // Render main content
    let content = if state.mode == Mode::Search {
        vec![
            Span::styled(" / ", colors.text_accent().add_modifier(Modifier::BOLD)),
            Span::styled(
                &state.search_query,
                colors.text().add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", colors.text_primary()),
            Span::styled("  (Enter: keep filter │ Esc: clear)", colors.text_muted()),
        ]
    } else if let Some(msg) = &state.status_message {
        vec![
            Span::styled(" ", Style::default()),
            Span::styled(msg, colors.text_secondary()),
        ]
    } else {
        vec![
            Span::styled(" ", Style::default()),
            Span::styled("/", colors.key_hint()),
            Span::styled(": search  ", colors.text_muted()),
            Span::styled("f", colors.key_hint()),
            Span::styled(": due filter  ", colors.text_muted()),
            Span::styled("Tab", colors.key_hint()),
            Span::styled(": focus  ", colors.text_muted()),
            Span::styled("?", colors.key_hint()),
            Span::styled(": help  ", colors.text_muted()),
            Span::styled("s", colors.key_hint()),
            Span::styled(": settings  ", colors.text_muted()),
            Span::styled("t", colors.key_hint()),
            Span::styled(": theme  ", colors.text_muted()),
            Span::styled("A", colors.key_hint()),
            Span::styled(": about  ", colors.text_muted()),
            Span::styled("q", colors.key_hint()),
            Span::styled(": quit", colors.text_muted()),
        ]
    };

    let status =
        Paragraph::new(Line::from(content)).style(Style::default().bg(colors.bg_secondary));
    frame.render_widget(status, content_area);

    if state.mode == Mode::Search {
        let query_width = unicode_width::UnicodeWidthStr::width(state.search_query.as_str()) as u16;
        frame.set_cursor_position((content_area.x + 3 + query_width, content_area.y));
    }

    // Render sync indicator separately (only this part updates during sync)
    if state.is_sync_enabled() {
        let sync_indicator = if state.sync_status.syncing {
            Span::styled(
                " ↻ Syncing... ",
                Style::default().fg(Color::Cyan).bg(colors.bg_secondary),
            )
        } else if let Some(ref error) = state.sync_status.last_error {
            // Truncate error for status bar, show abbreviated message
            let short_error = if error.len() > 20 {
                format!(" ⚠ {}… ", &error[..20])
            } else {
                format!(" ⚠ {} ", error)
            };
            Span::styled(
                short_error,
                Style::default().fg(Color::Red).bg(colors.bg_secondary),
            )
        } else if state.sync_status.last_sync.is_some() {
            Span::styled(
                " ☁ Synced ",
                Style::default().fg(Color::Green).bg(colors.bg_secondary),
            )
        } else {
            Span::styled(
                " S: sync ",
                Style::default().fg(colors.fg_muted).bg(colors.bg_secondary),
            )
        };

        let sync_widget = Paragraph::new(Line::from(vec![sync_indicator]))
            .style(Style::default().bg(colors.bg_secondary))
            .alignment(Alignment::Right);
        frame.render_widget(sync_widget, sync_area);
    }
}

/// Render help popup
fn render_help_popup(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Calculate popup size
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 34u16.min(area.height.saturating_sub(4));

    let popup_area = Rect {
        x: area.x + (area.width - popup_width) / 2,
        y: area.y + (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let help_content = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Navigation",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab    ", colors.key_hint()),
            Span::styled("Switch focus (sidebar / tasks)", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  s                  ", colors.key_hint()),
            Span::styled(
                "Open Settings (Lists, Tags, Sync, ...)",
                colors.text_accent(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  j/k or ↑/↓         ", colors.key_hint()),
            Span::styled("Navigate lists", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  g/G                ", colors.key_hint()),
            Span::styled("Go to first/last item", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  h/l or ←/→         ", colors.key_hint()),
            Span::styled("Focus sidebar/main", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Tasks View",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Space / x          ", colors.key_hint()),
            Span::styled("Toggle task complete", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  Enter / e          ", colors.key_hint()),
            Span::styled("Open / edit task", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  n                  ", colors.key_hint()),
            Span::styled("Create new task", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  e                  ", colors.key_hint()),
            Span::styled("Edit selected task", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  d                  ", colors.key_hint()),
            Span::styled("Delete selected task", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  p                  ", colors.key_hint()),
            Span::styled("Cycle priority", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  o                  ", colors.key_hint()),
            Span::styled("Open task URL", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  c                  ", colors.key_hint()),
            Span::styled("Toggle show completed", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  u                  ", colors.key_hint()),
            Span::styled("Undo last completed task", colors.text_accent()),
        ]),
        Line::from(vec![
            Span::styled("  R                  ", colors.key_hint()),
            Span::styled("Toggle Recently Closed view", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  /                  ", colors.key_hint()),
            Span::styled("Search / filter tasks live", colors.text_accent()),
        ]),
        Line::from(vec![
            Span::styled("  f / F  or  [ / ]   ", colors.key_hint()),
            Span::styled(
                "Cycle due-date filter (All / Overdue / Today / Week / No Date)",
                colors.text(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  General",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  s                  ", colors.key_hint()),
            Span::styled("Open settings", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  t                  ", colors.key_hint()),
            Span::styled("Open theme selector", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  A                  ", colors.key_hint()),
            Span::styled("About Tickit", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  r                  ", colors.key_hint()),
            Span::styled("Refresh data", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  ?                  ", colors.key_hint()),
            Span::styled("Toggle this help", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+s / S         ", colors.key_hint()),
            Span::styled("Sync with server (if configured)", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  q / Ctrl+c         ", colors.key_hint()),
            Span::styled("Quit application", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", colors.text_muted()),
            Span::styled("Esc", colors.key_hint()),
            Span::styled(" or ", colors.text_muted()),
            Span::styled("?", colors.key_hint()),
            Span::styled(" to close", colors.text_muted()),
        ]),
    ];

    let help = Paragraph::new(help_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(colors.block_focus())
                .style(Style::default().bg(colors.bg_secondary))
                .title(" ⌨ Keyboard Shortcuts ")
                .title_style(colors.text_primary()),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(help, popup_area);
}

/// Render theme picker
fn render_theme_picker(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = centered_rect(50, 70, frame.area());

    frame.render_widget(Clear, area);

    let themes = Theme::all();
    let items: Vec<ListItem> = themes
        .iter()
        .enumerate()
        .map(|(i, theme_name)| {
            let palette = theme_name.palette();
            let selected = i == state.theme_index;

            // Create color preview squares
            let preview = format!(
                "  {} {} ",
                if selected { "▸" } else { " " },
                theme_name.display_name()
            );

            let style = if selected {
                Style::default()
                    .fg(palette.accent)
                    .bg(palette.selection)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.fg)
            };

            ListItem::new(Line::from(vec![
                Span::styled(preview, style),
                Span::styled("█", Style::default().fg(palette.accent)),
                Span::styled("█", Style::default().fg(palette.secondary)),
                Span::styled("█", Style::default().fg(palette.success)),
                Span::styled("█", Style::default().fg(palette.warning)),
            ]))
        })
        .collect();

    let theme_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors.accent))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(colors.bg))
            .title(format!(
                " 🎨 Select Theme ({}/{}) ",
                state.theme_index + 1,
                themes.len()
            ))
            .title_bottom(Line::from(" ↑↓ navigate │ ↵ apply │ Esc cancel ").centered()),
    );

    frame.render_widget(theme_list, area);
}

/// Render settings dialog (tuios style)
fn render_settings_dialog(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Calculate popup size
    let popup_width = 80u16.min(area.width.saturating_sub(4));
    let popup_height = 22u16.min(area.height.saturating_sub(2));

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let is_editing = state.mode == Mode::SettingsInput;

    let footer_text = match state.settings_category {
        SettingsCategory::Lists => {
            " Tab: category │ j/k: select │ a: add │ e: edit │ d: delete │ Esc: close "
        }
        SettingsCategory::Tags => {
            " Tab: category │ j/k: select │ a: add │ e: edit │ d: delete │ Esc: close "
        }
        SettingsCategory::About => " Tab: category │ g: open github │ Esc: close ",
        _ => {
            if is_editing {
                " Type to edit │ ↵ save │ Esc cancel "
            } else {
                " Tab: category │ ↑↓: navigate │ ←→/↵: change │ Esc: close "
            }
        }
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors.accent))
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(colors.bg))
        .title_bottom(Line::from(footer_text).centered());

    let inner = outer_block.inner(popup_area);
    frame.render_widget(outer_block, popup_area);

    if inner.height < 4 || inner.width < 30 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top badge ("Settings")
            Constraint::Length(1), // Category navigation tabs
            Constraint::Length(1), // Thin divider
            Constraint::Min(1),    // Category Content Body
        ])
        .split(inner);

    // 1. Top Title Badge
    let badge_line = Line::from(vec![Span::styled(
        " Settings ",
        Style::default()
            .fg(colors.bg)
            .bg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(Paragraph::new(badge_line), chunks[0]);

    // 2. Category Tabs
    let mut cat_spans = Vec::new();
    cat_spans.push(Span::raw(" "));
    for cat in SettingsCategory::all() {
        let is_active = *cat == state.settings_category;
        if is_active {
            cat_spans.push(Span::styled(
                format!(" {} ", cat.name()),
                Style::default()
                    .fg(colors.bg)
                    .bg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            cat_spans.push(Span::styled(
                format!(" {} ", cat.name()),
                colors.text_muted(),
            ));
        }
        cat_spans.push(Span::raw(" "));
    }
    frame.render_widget(Paragraph::new(Line::from(cat_spans)), chunks[1]);

    // 3. Thin Horizontal Divider
    let divider = "─".repeat(inner.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(divider, colors.block())),
        chunks[2],
    );

    // 4. Content Body
    match state.settings_category {
        SettingsCategory::Appearance | SettingsCategory::Behavior | SettingsCategory::Sync => {
            render_settings_items_category(frame, state, chunks[3]);
        }
        SettingsCategory::Lists => {
            render_settings_lists_category(frame, state, chunks[3]);
        }
        SettingsCategory::Tags => {
            render_settings_tags_category(frame, state, chunks[3]);
        }
        SettingsCategory::About => {
            render_settings_about_category(frame, state, chunks[3]);
        }
    }
}

fn render_settings_items_category(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();
    let items = state.settings_category.items();
    let is_editing = state.mode == Mode::SettingsInput;

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let selected = i == state.settings_index;
            let prefix = if selected { "› " } else { "  " };

            let (widget_spans, widget_len) =
                get_settings_widget_spans(state, *item, is_editing, selected);

            let label_str = format!("{}{}", prefix, item.label());
            let label_len = unicode_width::UnicodeWidthStr::width(label_str.as_str());

            let padding = (area.width as usize).saturating_sub(label_len + widget_len + 1);

            let label_style = if selected {
                colors.selected().add_modifier(Modifier::BOLD)
            } else {
                colors.text()
            };

            let mut row_spans = vec![Span::styled(label_str, label_style)];
            row_spans.push(Span::raw(" ".repeat(padding)));
            row_spans.extend(widget_spans);

            let row_line = Line::from(row_spans);
            if selected {
                ListItem::new(row_line).style(colors.selected())
            } else {
                ListItem::new(row_line)
            }
        })
        .collect();

    frame.render_widget(List::new(list_items), area);
}

fn get_settings_widget_spans<'a>(
    state: &'a AppState,
    item: SettingsItem,
    is_editing: bool,
    selected: bool,
) -> (Vec<Span<'a>>, usize) {
    let colors = state.theme.colors();

    match item {
        SettingsItem::Theme => {
            let name = state.theme.name();
            let arrow_style = if selected {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                colors.text_muted()
            };
            let text_style = if selected {
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                colors.text()
            };
            let text_len = unicode_width::UnicodeWidthStr::width(name) + 4;
            let spans = vec![
                Span::styled("‹ ", arrow_style),
                Span::styled(name.to_string(), text_style),
                Span::styled(" ›", arrow_style),
            ];
            (spans, text_len)
        }
        SettingsItem::ShowCompletedDefault => {
            let on = state.config.show_completed;
            make_toggle_spans(on, &colors)
        }
        SettingsItem::EnterCompletes => {
            let label = if state.config.enter_completes {
                "Complete task"
            } else {
                "Open / Edit"
            };
            let arrow_style = if selected {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                colors.text_muted()
            };
            let text_style = if selected {
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                colors.text()
            };
            let text_len = unicode_width::UnicodeWidthStr::width(label) + 4;
            let spans = vec![
                Span::styled("‹ ", arrow_style),
                Span::styled(label.to_string(), text_style),
                Span::styled(" ›", arrow_style),
            ];
            (spans, text_len)
        }
        SettingsItem::Notifications => {
            let on = state.config.notifications;
            make_toggle_spans(on, &colors)
        }
        SettingsItem::SyncEnabled => {
            let on = state.config.sync.enabled;
            make_toggle_spans(on, &colors)
        }
        SettingsItem::SyncServer => {
            if is_editing && state.settings_editing == Some(item) {
                render_input_spans(&state.input_buffer, state.cursor_pos, &colors)
            } else {
                let text = state.config.sync.server.as_deref().unwrap_or("(not set)");
                make_bracket_spans(text, &colors)
            }
        }
        SettingsItem::SyncToken => {
            if is_editing && state.settings_editing == Some(item) {
                render_input_spans(&state.input_buffer, state.cursor_pos, &colors)
            } else {
                let text = if state.config.sync.token.is_some() {
                    "••••••••"
                } else {
                    "(not set)"
                };
                make_bracket_spans(text, &colors)
            }
        }
        SettingsItem::SyncInterval => {
            let text = if state.config.sync.interval_secs == 0 {
                "manual".to_string()
            } else {
                format!("{}s", state.config.sync.interval_secs)
            };
            let arrow_style = if selected {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                colors.text_muted()
            };
            let text_style = if selected {
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                colors.text()
            };
            let text_len = unicode_width::UnicodeWidthStr::width(text.as_str()) + 4;
            let spans = vec![
                Span::styled("‹ ", arrow_style),
                Span::styled(text, text_style),
                Span::styled(" ›", arrow_style),
            ];
            (spans, text_len)
        }
    }
}

fn make_toggle_spans<'a>(on: bool, colors: &crate::theme::ThemeColors) -> (Vec<Span<'a>>, usize) {
    if on {
        let spans = vec![
            Span::styled("[ ", colors.text_muted()),
            Span::styled(
                "on",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ]", colors.text_muted()),
        ];
        (spans, 6)
    } else {
        let spans = vec![
            Span::styled("[ ", colors.text_muted()),
            Span::styled("off", colors.text_muted()),
            Span::styled(" ]", colors.text_muted()),
        ];
        (spans, 7)
    }
}

fn make_bracket_spans<'a>(
    text: &'a str,
    colors: &crate::theme::ThemeColors,
) -> (Vec<Span<'a>>, usize) {
    let len = unicode_width::UnicodeWidthStr::width(text) + 4;
    let spans = vec![
        Span::styled("[ ", colors.text_muted()),
        Span::styled(text, colors.text_secondary()),
        Span::styled(" ]", colors.text_muted()),
    ];
    (spans, len)
}

fn render_input_spans<'a>(
    buf: &'a str,
    cursor_pos: usize,
    colors: &crate::theme::ThemeColors,
) -> (Vec<Span<'a>>, usize) {
    let pos = cursor_pos.min(buf.len());
    let safe_pos = if buf.is_char_boundary(pos) {
        pos
    } else {
        buf[..pos]
            .char_indices()
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0)
    };
    let before = &buf[..safe_pos];
    let after = &buf[safe_pos..];
    let text = format!("{}▏{}", before, after);
    let len = unicode_width::UnicodeWidthStr::width(text.as_str()) + 4;
    let spans = vec![
        Span::styled("[ ", Style::default().fg(colors.accent)),
        Span::styled(
            text,
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ]", Style::default().fg(colors.accent)),
    ];
    (spans, len)
}

fn render_settings_lists_category(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let mut lines = Vec::new();
    lines.push(ListItem::new(Line::from(vec![Span::styled(
        "   Icon Name                         Default       Tasks",
        colors.text_secondary().add_modifier(Modifier::BOLD),
    )])));

    for (i, list) in state.lists.iter().enumerate() {
        let selected = i == state.list_index;
        let prefix = if selected { " › " } else { "   " };
        let icon_str = format!("{:<4}", list.icon);
        let name_trimmed = if list.name.len() > 26 {
            format!("{}…", &list.name[..25])
        } else {
            list.name.clone()
        };
        let name_str = format!("{:<28}", name_trimmed);
        let def_str = if list.is_inbox {
            format!("{:<14}", "[default]")
        } else {
            format!("{:<14}", "")
        };
        let count = state
            .tasks
            .iter()
            .filter(|t| t.list_id == list.id && !t.completed)
            .count();
        let count_str = format!("{} active", count);

        let row_style = if selected {
            colors.selected().add_modifier(Modifier::BOLD)
        } else {
            colors.text()
        };

        lines.push(
            ListItem::new(Line::from(vec![
                Span::styled(prefix, row_style),
                Span::styled(icon_str, row_style),
                Span::styled(name_str, row_style),
                Span::styled(
                    def_str,
                    if list.is_inbox {
                        colors.text_info()
                    } else {
                        colors.text_muted()
                    },
                ),
                Span::styled(count_str, colors.text_muted()),
            ]))
            .style(if selected {
                colors.selected()
            } else {
                Style::default()
            }),
        );
    }

    if state.lists.is_empty() {
        lines.push(ListItem::new(Line::from(Span::styled(
            "   (No lists created yet)",
            colors.text_muted(),
        ))));
    }

    frame.render_widget(List::new(lines), area);
}

fn render_settings_tags_category(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let mut lines = Vec::new();
    lines.push(ListItem::new(Line::from(vec![Span::styled(
        "   Color       Tag Name                        Tasks",
        colors.text_secondary().add_modifier(Modifier::BOLD),
    )])));

    for (i, tag) in state.tags.iter().enumerate() {
        let selected = i == state.tag_index;
        let prefix = if selected { " › " } else { "   " };
        let tag_color = Color::from_u32(
            u32::from_str_radix(tag.color.trim_start_matches('#'), 16).unwrap_or(0x3498db),
        );
        let hex_str = format!(" {:<10}", tag.color);
        let name_trimmed = if tag.name.len() > 30 {
            format!("{}…", &tag.name[..29])
        } else {
            tag.name.clone()
        };
        let name_str = format!("{:<32}", name_trimmed);
        let count = state
            .tasks
            .iter()
            .filter(|t| t.tag_ids.contains(&tag.id) && !t.completed)
            .count();
        let count_str = format!("{} active", count);

        let row_style = if selected {
            colors.selected().add_modifier(Modifier::BOLD)
        } else {
            colors.text()
        };

        lines.push(
            ListItem::new(Line::from(vec![
                Span::styled(prefix, row_style),
                Span::styled(
                    "■",
                    Style::default().fg(tag_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(hex_str, colors.text_muted()),
                Span::styled(name_str, row_style),
                Span::styled(count_str, colors.text_muted()),
            ]))
            .style(if selected {
                colors.selected()
            } else {
                Style::default()
            }),
        );
    }

    if state.tags.is_empty() {
        lines.push(ListItem::new(Line::from(Span::styled(
            "   (No tags created yet)",
            colors.text_muted(),
        ))));
    }

    frame.render_widget(List::new(lines), area);
}

fn render_settings_about_category(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let total_tasks = state.tasks.len();
    let completed_tasks = state.tasks.iter().filter(|t| t.completed).count();
    let active_tasks = total_tasks.saturating_sub(completed_tasks);

    let sync_status_str = if !state.config.sync.enabled {
        "Disabled".to_string()
    } else if state.sync_status.syncing {
        "Syncing...".to_string()
    } else if state.sync_status.last_error.is_some() {
        "Error (see status)".to_string()
    } else {
        "Connected".to_string()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} Tickit ", ICON), colors.logo_style_primary()),
            Span::styled(
                format!("v{}", env!("CARGO_PKG_VERSION")),
                colors.text_accent().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " — Fast, beautiful terminal task manager",
                colors.text_muted(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tasks:        ", colors.text_secondary()),
            Span::styled(format!("{} total", total_tasks), colors.text()),
            Span::styled(
                format!(" ({} active, {} completed)", active_tasks, completed_tasks),
                colors.text_muted(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Lists / Tags: ", colors.text_secondary()),
            Span::styled(
                format!("{} lists, {} tags", state.lists.len(), state.tags.len()),
                colors.text(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Sync Status:  ", colors.text_secondary()),
            Span::styled(sync_status_str, colors.text_info()),
        ]),
        Line::from(vec![
            Span::styled("  Repository:   ", colors.text_secondary()),
            Span::styled(
                "https://github.com/ricardodantas/tickit",
                colors.text_accent(),
            ),
            Span::styled(" (press 'g' to open)", colors.key_hint()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Shortcuts Cheatsheet",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Tab", colors.key_hint()),
            Span::styled(": focus sidebar/tasks  ", colors.text_muted()),
            Span::styled("/", colors.key_hint()),
            Span::styled(": live search  ", colors.text_muted()),
            Span::styled("f", colors.key_hint()),
            Span::styled(": due filter  ", colors.text_muted()),
            Span::styled("n", colors.key_hint()),
            Span::styled(": new task", colors.text_muted()),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

/// Render confirmation dialog
fn render_confirm_dialog(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = centered_rect(50, 20, frame.area());

    frame.render_widget(Clear, area);

    let text = vec![
        Line::from(""),
        Line::from(state.confirm_message.as_str()),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y  ", colors.key_hint()),
            Span::raw("Yes  "),
            Span::styled("  n  ", colors.key_hint()),
            Span::raw("No"),
        ]),
    ];

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Confirm ")
                .title_style(colors.text_warning())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.block_focus()),
        )
        .alignment(Alignment::Center);

    frame.render_widget(dialog, area);
}

/// Render task editor
fn render_task_editor(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = centered_rect(60, 70, frame.area());

    frame.render_widget(Clear, area);

    let title = if state.mode == Mode::AddTask {
        " New Task "
    } else {
        " Edit Task "
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title input
            Constraint::Min(6),    // Description input (multi-line)
            Constraint::Length(3), // Due Date input
            Constraint::Length(3), // Priority
            Constraint::Length(3), // List
            Constraint::Length(5), // Tags (expanded)
            Constraint::Length(1), // Help
        ])
        .split(area);

    // Title field
    let title_focused = state.editor_field == EditorField::Title;
    let title_style = if title_focused {
        colors.block_focus()
    } else {
        colors.block()
    };

    // Show the correct title value depending on focus
    let title_display = if title_focused {
        state.input_buffer.as_str()
    } else {
        state.editor_title_buffer.as_str()
    };

    let title_input = Paragraph::new(title_display).block(
        Block::default()
            .title(" Title ")
            .borders(Borders::ALL)
            .border_style(title_style),
    );
    frame.render_widget(title_input, chunks[0]);

    if title_focused && !state.editor_adding_tag {
        let (col, _) = state.input_cursor_display_offset();
        frame.set_cursor_position((chunks[0].x + col + 1, chunks[0].y + 1));
    }

    // Description field
    let desc_focused = state.editor_field == EditorField::Description;
    let desc_style = if desc_focused {
        colors.block_focus()
    } else {
        colors.block()
    };
    let desc_display = if desc_focused {
        state.input_buffer.as_str()
    } else {
        state.editor_description_buffer.as_str()
    };
    let desc_input = Paragraph::new(desc_display)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Description (optional - Shift+Enter for newline) ")
                .borders(Borders::ALL)
                .border_style(desc_style),
        );
    frame.render_widget(desc_input, chunks[1]);

    if desc_focused && !state.editor_adding_tag {
        let (col, row) = state.input_cursor_display_offset();
        frame.set_cursor_position((chunks[1].x + col + 1, chunks[1].y + row + 1));
    }

    // Due Date field & Quick Presets Picker
    let due_focused = state.editor_field == EditorField::DueDate;
    let presets_focused = state.editor_field == EditorField::DuePresets;

    let due_style = if due_focused {
        colors.block_focus()
    } else {
        colors.block()
    };
    let presets_style = if presets_focused {
        colors.block_focus()
    } else {
        colors.block()
    };

    let due_display = if due_focused {
        state.input_buffer.as_str()
    } else {
        state.editor_due_date_buffer.as_str()
    };

    let due_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(16), // text input box for YYYY-MM-DD
            Constraint::Min(0),     // quick picker
        ])
        .split(chunks[2]);

    let due_placeholder = if due_display.is_empty() {
        "YYYY-MM-DD"
    } else {
        due_display
    };
    let due_input = Paragraph::new(due_placeholder)
        .style(if due_display.is_empty() && !due_focused {
            colors.text_muted()
        } else {
            colors.text()
        })
        .block(
            Block::default()
                .title(" Due Date ")
                .borders(Borders::ALL)
                .border_style(due_style),
        );
    frame.render_widget(due_input, due_row[0]);

    if due_focused && !state.editor_adding_tag {
        let (col, _) = state.input_cursor_display_offset();
        frame.set_cursor_position((due_row[0].x + col + 1, due_row[0].y + 1));
    }

    // Render Tag-like Quick Date Picker
    let now = chrono::Utc::now();
    let mut picker_spans = Vec::new();
    let is_wide = due_row[1].width >= 62;
    let is_compact = due_row[1].width < 48;

    picker_spans.push(Span::raw(" "));
    for (i, preset) in QuickDatePreset::all().iter().enumerate() {
        let is_applied = preset.to_date_string(now) == due_display.trim();
        let is_cursor = presets_focused && state.editor_preset_index == i;

        let marker = if is_cursor { "►" } else { " " };
        let radio = if is_applied { "◉" } else { "○" };
        let label = if is_compact {
            match i {
                0 => "Tod",
                1 => "Tom",
                2 => "Sat",
                3 => "Mon",
                4 => "2W",
                _ => "",
            }
        } else if is_wide {
            preset.label()
        } else {
            match i {
                0 => "Today",
                1 => "Tmrw",
                2 => "Wknd",
                3 => "NextWk",
                4 => "+2W",
                _ => "",
            }
        };

        let style = if is_cursor {
            colors.selected().add_modifier(Modifier::BOLD)
        } else if is_applied {
            colors.text_success()
        } else {
            colors.text()
        };

        picker_spans.push(Span::styled(
            format!("{}{} {}:{} ", marker, radio, i + 1, label),
            style,
        ));
    }

    // Clear / None option
    let is_none_applied = due_display.trim().is_empty();
    let is_none_cursor = presets_focused && state.editor_preset_index == 5;
    let none_marker = if is_none_cursor { "►" } else { " " };
    let none_radio = if is_none_applied { "◉" } else { "○" };
    let none_style = if is_none_cursor {
        colors.selected().add_modifier(Modifier::BOLD)
    } else if is_none_applied {
        colors.text_muted()
    } else {
        colors.text()
    };
    picker_spans.push(Span::styled(
        format!("{}{} c:None", none_marker, none_radio),
        none_style,
    ));

    let picker_title = if presets_focused {
        " Quick Picker (Space: pick, 1-5, ←/→, c: clear) "
    } else {
        " Quick Date Picker (Tab to focus) "
    };

    let presets_widget = Paragraph::new(Line::from(picker_spans)).block(
        Block::default()
            .title(picker_title)
            .borders(Borders::ALL)
            .border_style(presets_style),
    );
    frame.render_widget(presets_widget, due_row[1]);

    // Priority field
    let priority_focused = state.editor_field == EditorField::Priority;
    let priority_style = if priority_focused {
        colors.block_focus()
    } else {
        colors.block()
    };
    let priority_text = format!(
        "{} {}",
        state.editor_priority.icon(),
        state.editor_priority.name()
    );
    let priority_input = Paragraph::new(priority_text).block(
        Block::default()
            .title(" Priority (j/k to change) ")
            .borders(Borders::ALL)
            .border_style(priority_style),
    );
    frame.render_widget(priority_input, chunks[3]);

    // List field
    let list_focused = state.editor_field == EditorField::List;
    let list_style = if list_focused {
        colors.block_focus()
    } else {
        colors.block()
    };
    let list_text = state
        .lists
        .get(state.editor_list_index)
        .map(|l| format!("{} {}", l.icon, l.name))
        .unwrap_or_else(|| "📥 Inbox".to_string());
    let list_input = Paragraph::new(list_text).block(
        Block::default()
            .title(" List (j/k to change) ")
            .borders(Borders::ALL)
            .border_style(list_style),
    );
    frame.render_widget(list_input, chunks[4]);

    // Tags field - show as selectable list
    let tags_focused = state.editor_field == EditorField::Tags;
    let tags_style = if tags_focused {
        colors.block_focus()
    } else {
        colors.block()
    };

    let mut tag_items: Vec<ListItem> = state
        .tags
        .iter()
        .enumerate()
        .map(|(i, tag)| {
            let is_selected = state.editor_tag_indices.contains(&i);
            let is_cursor = tags_focused && i == state.editor_tag_cursor;

            let checkbox = if is_selected { "☑" } else { "☐" };
            let marker = if is_cursor { "► " } else { "  " };

            let style = if is_cursor {
                colors.selected()
            } else {
                colors.text()
            };

            let tag_color = parse_hex_color(&tag.color).unwrap_or(colors.accent);

            ListItem::new(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(
                    format!("{} ", checkbox),
                    if is_selected {
                        colors.text_success()
                    } else {
                        colors.text_muted()
                    },
                ),
                Span::styled("● ", Style::default().fg(tag_color)),
                Span::styled(&tag.name, style),
            ]))
        })
        .collect();

    // Add "Add new tag" option
    let add_new_cursor = tags_focused && state.editor_tag_cursor == state.tags.len();
    let add_style = if add_new_cursor {
        colors.selected()
    } else {
        colors.text_muted()
    };
    let add_marker = if add_new_cursor { "► " } else { "  " };

    if state.editor_adding_tag {
        // Show input field for new tag
        tag_items.push(ListItem::new(Line::from(vec![
            Span::styled(add_marker, add_style),
            Span::styled("+ ", colors.text_success()),
            Span::styled(&state.editor_new_tag_buffer, colors.text()),
            Span::styled("_", colors.text_primary()), // cursor
        ])));
    } else {
        tag_items.push(ListItem::new(Line::from(vec![
            Span::styled(add_marker, add_style),
            Span::styled("+ Add new tag...", add_style),
        ])));
    }

    let tags_list = List::new(tag_items).block(
        Block::default()
            .title(" Tags (Space: toggle, n: new) ")
            .borders(Borders::ALL)
            .border_style(tags_style),
    );
    frame.render_widget(tags_list, chunks[5]);

    // Help text
    let help_text = if state.editor_adding_tag {
        "Enter: save tag │ Esc: cancel"
    } else if state.editor_field == EditorField::DuePresets {
        "Space: pick date │ 1-5: quick pick │ ←/→: navigate │ c: clear │ Enter: save"
    } else if state.editor_field == EditorField::DueDate {
        "Tab: quick picker │ Type date or 'today' │ Enter: save │ Esc: cancel"
    } else {
        "Tab: next field │ Enter: save │ Esc: cancel"
    };
    let help = Paragraph::new(help_text)
        .style(colors.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[6]);

    // Outer block
    let outer = Block::default()
        .title(title)
        .title_style(colors.text_primary())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.block_focus())
        .style(Style::default().bg(colors.bg));
    frame.render_widget(outer, area);
}

/// Render simple name editor (for lists and tags)
fn render_simple_editor(frame: &mut Frame, state: &AppState, item_type: &str) {
    let colors = state.theme.colors();
    let area = centered_rect(50, 25, frame.area());

    frame.render_widget(Clear, area);

    let title = if state.mode == Mode::AddList || state.mode == Mode::AddTag {
        format!(" New {} ", item_type)
    } else {
        format!(" Edit {} ", item_type)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Name input
            Constraint::Min(1),    // Spacer
            Constraint::Length(1), // Help
        ])
        .split(area);

    let input = Paragraph::new(state.input_buffer.as_str()).block(
        Block::default()
            .title(" Name ")
            .borders(Borders::ALL)
            .border_style(colors.block_focus()),
    );
    frame.render_widget(input, chunks[0]);

    let (col, _) = state.input_cursor_display_offset();
    frame.set_cursor_position((chunks[0].x + col + 1, chunks[0].y + 1));

    let help = Paragraph::new("Enter: save │ Esc: cancel")
        .style(colors.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[2]);

    let outer = Block::default()
        .title(title)
        .title_style(colors.text_primary())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.block_focus())
        .style(Style::default().bg(colors.bg));
    frame.render_widget(outer, area);
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Render about dialog
fn render_about_dialog(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = centered_rect(70, 60, frame.area());

    frame.render_widget(Clear, area);

    let version = env!("CARGO_PKG_VERSION");
    let repo = "https://github.com/ricardodantas/tickit";

    let logo = [
        "████████╗██╗ ██████╗██╗  ██╗██╗████████╗",
        "╚══██╔══╝██║██╔════╝██║ ██╔╝██║╚══██╔══╝",
        "   ██║   ██║██║     █████╔╝ ██║   ██║   ",
        "   ██║   ██║██║     ██╔═██╗ ██║   ██║   ",
        "   ██║   ██║╚██████╗██║  ██╗██║   ██║   ",
        "   ╚═╝   ╚═╝ ╚═════╝╚═╝  ╚═╝╚═╝   ╚═╝   ",
    ];

    let mut lines: Vec<Line> = logo
        .iter()
        .map(|line| Line::from(Span::styled(*line, Style::default().fg(colors.primary))))
        .collect();

    lines.extend([
        Line::from(""),
        Line::from(Span::styled(
            "✓ get stuff done",
            Style::default()
                .fg(colors.fg)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "A stunning terminal-based task manager",
            colors.text_muted(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Version: ", colors.text_muted()),
            Span::styled(
                version,
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ]);

    // Show update available
    if let Some(new_version) = &state.update_available {
        lines.push(Line::from(vec![
            Span::styled("Update: ", colors.text_muted()),
            Span::styled(
                format!("v{} available!", new_version),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    lines.extend([
        Line::from(""),
        Line::from(vec![
            Span::styled("Author: ", colors.text_muted()),
            Span::styled("Ricardo Dantas", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("License: ", colors.text_muted()),
            Span::styled("MIT", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("Repo: ", colors.text_muted()),
            Span::styled(repo, Style::default().fg(colors.primary)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Built with Rust 🦀 + Ratatui",
            colors.text_muted().add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
    ]);

    // Build action hints line
    let mut actions: Vec<Span> = vec![
        Span::styled(
            " [G] ",
            Style::default()
                .fg(colors.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Open GitHub"),
    ];

    if state.update_available.is_some() {
        actions.extend([
            Span::raw("  "),
            Span::styled(
                " [U] ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Update"),
        ]);
    }

    actions.extend([
        Span::raw("  "),
        Span::styled(" [Esc] ", colors.text_muted()),
        Span::raw("Close"),
    ]);

    lines.push(Line::from(actions));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.primary))
            .style(Style::default().bg(colors.bg))
            .title(" ✓ About Tickit ")
            .title_style(
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(paragraph, area);
}

/// Parse a hex color string
fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn render_update_banner(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    if let Some(ref latest) = state.update_available {
        let pm = crate::detect_package_manager();
        let banner = Paragraph::new(Line::from(vec![
            Span::styled("  ⬆️  ", Style::default().fg(colors.warning)),
            Span::styled("Update available: ", colors.text()),
            Span::styled(
                format!("v{}", latest),
                Style::default()
                    .fg(colors.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (current: ", colors.text_muted()),
            Span::styled(format!("v{})", crate::VERSION), colors.text_muted()),
            Span::styled(" — Press ", colors.text_muted()),
            Span::styled(
                "[U]",
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" to update via {}", pm.name()), colors.text_muted()),
        ]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.warning))
                .style(Style::default().bg(colors.bg)),
        );

        frame.render_widget(banner, area);
    }
}

fn render_update_confirm_dialog(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    let popup_width = 50u16;
    let popup_height = 9u16;
    let popup_area = Rect {
        x: area.width.saturating_sub(popup_width) / 2,
        y: area.height.saturating_sub(popup_height) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    frame.render_widget(Clear, popup_area);

    let latest = state.update_available.as_deref().unwrap_or("unknown");
    let pm = crate::detect_package_manager();

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Update to ", colors.text()),
            Span::styled(
                format!("v{}", latest),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Command: ", colors.text_muted()),
            Span::styled(pm.update_command(), Style::default().fg(colors.primary)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [Y] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Yes, update"),
            Span::raw("    "),
            Span::styled(" [N/Esc] ", colors.text_muted()),
            Span::raw("Cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(colors.bg))
            .title(" ⬆️ Update Tickit ")
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(paragraph, popup_area);
}

fn render_updating_overlay(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Dim the background with semi-transparent overlay
    let overlay = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(overlay, area);

    // Centered modal
    let popup_width = 40u16;
    let popup_height = 5u16;

    // Calculate centered position
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;

    let popup_area = Rect {
        x,
        y,
        width: popup_width.min(area.width.saturating_sub(x)),
        height: popup_height.min(area.height.saturating_sub(y)),
    };

    // Clear the popup area first to ensure clean rendering
    frame.render_widget(Clear, popup_area);

    let msg = state
        .update_result
        .as_deref()
        .unwrap_or("Updating... please wait");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("⏳ {}", msg),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center).block(
        Block::default()
            .title(" Update in Progress ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(colors.bg)),
    );

    frame.render_widget(paragraph, popup_area);
}

//! Output panel module that displays stdout/stderr and accepts stdin
//! from flow execution in a tabbed view below the canvas.

use iced::widget::{button, container, scrollable, text, text_input, Column, Row, Space};
use iced::{Color, Element, Fill, Length, Theme};

/// Height of the output panel in pixels.
const PANEL_HEIGHT: f32 = 200.0;

/// Which tab is currently active in the output panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputTab {
    Stdout,
    Stderr,
    Stdin,
}

/// Messages produced by the output panel.
#[derive(Debug, Clone)]
pub(crate) enum OutputMessage {
    /// Switch to a different tab.
    SelectTab(OutputTab),
    /// Close the output panel.
    Close,
    /// Text typed into the stdin input field.
    StdinInput(String),
    /// User pressed Enter to submit stdin line.
    StdinSubmit,
    /// User clicked "Send EOF" to close stdin (signals end of input).
    StdinEof,
}

/// State for the output panel.
pub(crate) struct OutputPanel {
    /// Which tab is active
    active_tab: OutputTab,
    /// Lines captured from stdout
    pub stdout: Vec<String>,
    /// Lines captured from stderr
    pub stderr: Vec<String>,
    /// Lines submitted to stdin (history)
    pub stdin_history: Vec<String>,
    /// Current text in the stdin input field
    stdin_text: String,
    /// Cursor into stdin_history for lines not yet consumed by the coordinator
    stdin_cursor: usize,
    /// Whether the panel is visible
    pub visible: bool,
    /// Whether a flow process is currently running
    pub running: bool,
}

impl Default for OutputPanel {
    fn default() -> Self {
        Self {
            active_tab: OutputTab::Stdout,
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdin_history: Vec::new(),
            stdin_text: String::new(),
            stdin_cursor: 0,
            visible: false,
            running: false,
        }
    }
}

impl OutputPanel {
    /// Returns true if the panel should be shown.
    pub(crate) fn should_show(&self) -> bool {
        self.visible && (self.running || !self.stdout.is_empty() || !self.stderr.is_empty())
    }

    /// Clear all output for a new run.
    pub(crate) fn clear_for_run(&mut self) {
        self.stdout.clear();
        self.stderr.clear();
        // Don't clear stdin_history — allow pre-typing input before running
        self.stdin_cursor = 0;
        self.stdin_text.clear();
        self.active_tab = OutputTab::Stdout;
    }

    /// Take the next unconsumed stdin line (for GetLine requests from coordinator).
    pub(crate) fn take_stdin_line(&mut self) -> Option<String> {
        if let Some(line) = self.stdin_history.get(self.stdin_cursor) {
            let result = line.clone();
            self.stdin_cursor += 1;
            Some(result)
        } else {
            None
        }
    }

    /// Take all unconsumed stdin lines (for GetStdin requests from coordinator).
    pub(crate) fn take_all_stdin(&mut self) -> Option<String> {
        let remaining: Vec<&str> = self
            .stdin_history
            .iter()
            .skip(self.stdin_cursor)
            .map(|s| s.as_str())
            .collect();
        if remaining.is_empty() {
            None
        } else {
            self.stdin_cursor = self.stdin_history.len();
            Some(remaining.join("\n"))
        }
    }

    /// Get the current stdin cursor position.
    pub(crate) fn stdin_cursor(&self) -> usize {
        self.stdin_cursor
    }

    /// Advance the stdin cursor to the end (marks all lines as consumed).
    pub(crate) fn advance_stdin_cursor(&mut self) {
        self.stdin_cursor = self.stdin_history.len();
    }

    /// Handle an output panel message.
    /// Returns `Some(text)` when a stdin line should be sent to the child process.
    pub(crate) fn update(&mut self, message: &OutputMessage) -> Option<String> {
        match message {
            OutputMessage::SelectTab(tab) => {
                self.active_tab = *tab;
                None
            }
            OutputMessage::Close => {
                self.visible = false;
                None
            }
            OutputMessage::StdinInput(text) => {
                self.stdin_text = text.clone();
                None
            }
            OutputMessage::StdinSubmit => {
                if self.stdin_text.is_empty() {
                    return None;
                }
                let line = self.stdin_text.clone();
                self.stdin_history.push(line.clone());
                self.stdin_text.clear();
                Some(line)
            }
            OutputMessage::StdinEof => None,
        }
    }

    /// Render the output panel as an iced Element.
    pub(crate) fn view(&self) -> Element<'_, OutputMessage> {
        let mut col = Column::new().spacing(0);

        // Tab bar with close button
        let stdout_label = format!("Stdout ({})", self.stdout.len());
        let stderr_label = format!("Stderr ({})", self.stderr.len());

        let stdout_btn = button(text(stdout_label).size(12).center())
            .padding(iced::Padding {
                top: 4.0,
                right: 12.0,
                bottom: 4.0,
                left: 12.0,
            })
            .on_press(OutputMessage::SelectTab(OutputTab::Stdout))
            .style(if self.active_tab == OutputTab::Stdout {
                button::primary
            } else {
                button::secondary
            });

        let stderr_btn = button(text(stderr_label).size(12).center())
            .padding(iced::Padding {
                top: 4.0,
                right: 12.0,
                bottom: 4.0,
                left: 12.0,
            })
            .on_press(OutputMessage::SelectTab(OutputTab::Stderr))
            .style(if self.active_tab == OutputTab::Stderr {
                if self.stderr.is_empty() {
                    button::secondary
                } else {
                    button::danger
                }
            } else {
                button::secondary
            });

        let stdin_label = if self.running { "Stdin" } else { "Stdin" };
        let stdin_btn = button(text(stdin_label).size(12).center())
            .padding(iced::Padding {
                top: 4.0,
                right: 12.0,
                bottom: 4.0,
                left: 12.0,
            })
            .on_press(OutputMessage::SelectTab(OutputTab::Stdin))
            .style(if self.active_tab == OutputTab::Stdin {
                button::primary
            } else {
                button::secondary
            });

        let close_btn = button(text("X").size(11).center())
            .on_press(OutputMessage::Close)
            .style(button::text)
            .padding(4);

        let tab_bar = container(
            Row::new()
                .spacing(2)
                .push(stdout_btn)
                .push(stderr_btn)
                .push(stdin_btn)
                .push(Space::new().width(Fill))
                .push(close_btn),
        )
        .width(Fill)
        .padding(iced::Padding {
            top: 2.0,
            right: 4.0,
            bottom: 2.0,
            left: 4.0,
        })
        .style(|_theme: &Theme| container::Style {
            border: iced::Border {
                color: Color::from_rgb(0.3, 0.3, 0.3),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        col = col.push(tab_bar);

        // Tab content
        match self.active_tab {
            OutputTab::Stdout | OutputTab::Stderr => {
                let lines: &[String] = match self.active_tab {
                    OutputTab::Stdout => &self.stdout,
                    OutputTab::Stderr => &self.stderr,
                    OutputTab::Stdin => unreachable!(),
                };

                let text_color = match self.active_tab {
                    OutputTab::Stdout => Color::WHITE,
                    OutputTab::Stderr => Color::from_rgb(1.0, 0.5, 0.5),
                    OutputTab::Stdin => unreachable!(),
                };

                let mut content_col = Column::new().spacing(1).padding(6).width(Fill);
                if lines.is_empty() {
                    let label = match self.active_tab {
                        OutputTab::Stdout => "No stdout output",
                        OutputTab::Stderr => "No stderr output",
                        OutputTab::Stdin => unreachable!(),
                    };
                    content_col = content_col
                        .push(text(label).size(11).color(Color::from_rgb(0.5, 0.5, 0.5)));
                } else {
                    for line in lines {
                        content_col =
                            content_col.push(text(line.clone()).size(11).color(text_color));
                    }
                }

                let content_area =
                    container(scrollable(content_col).width(Fill).height(PANEL_HEIGHT))
                        .width(Fill)
                        .style(|_theme: &Theme| container::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(
                                0.08, 0.08, 0.08,
                            ))),
                            ..Default::default()
                        });

                col = col.push(content_area);
            }
            OutputTab::Stdin => {
                // History of submitted lines — always visible, like flowrgui
                let history_col =
                    Column::with_children(self.stdin_history.iter().cloned().map(|line| {
                        text(line)
                            .size(11)
                            .color(Color::from_rgb(0.6, 0.9, 0.6))
                            .into()
                    }))
                    .width(Fill)
                    .padding(1);

                let history_area = container(
                    scrollable(history_col)
                        .width(Fill)
                        .height(PANEL_HEIGHT - 36.0),
                )
                .width(Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.08))),
                    ..Default::default()
                });

                col = col.push(history_area);

                // Text input — only active when flow is running
                let mut input = text_input(
                    if self.running {
                        "Type input and press Enter..."
                    } else {
                        "Run a flow to enable stdin"
                    },
                    &self.stdin_text,
                )
                .size(12)
                .padding(8)
                .width(Fill);

                if self.running {
                    input = input
                        .on_input(OutputMessage::StdinInput)
                        .on_submit(OutputMessage::StdinSubmit);
                }

                let mut input_row = Row::new().spacing(4).push(input);

                if self.running {
                    let eof_btn = button(text("Send EOF").size(11).center())
                        .on_press(OutputMessage::StdinEof)
                        .style(button::secondary)
                        .padding(iced::Padding {
                            top: 6.0,
                            right: 10.0,
                            bottom: 6.0,
                            left: 10.0,
                        });
                    input_row = input_row.push(eof_btn);
                }

                col = col.push(input_row);
            }
        }

        container(col).width(Fill).height(Length::Shrink).into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_panel_is_hidden() {
        let panel = OutputPanel::default();
        assert!(!panel.visible);
        assert!(!panel.should_show());
    }

    #[test]
    fn should_show_when_running() {
        let mut panel = OutputPanel::default();
        panel.visible = true;
        panel.running = true;
        assert!(panel.should_show());
    }

    #[test]
    fn should_show_with_stdout() {
        let mut panel = OutputPanel::default();
        panel.visible = true;
        panel.stdout.push("hello".into());
        assert!(panel.should_show());
    }

    #[test]
    fn should_show_with_stderr() {
        let mut panel = OutputPanel::default();
        panel.visible = true;
        panel.stderr.push("error".into());
        assert!(panel.should_show());
    }

    #[test]
    fn clear_for_run_resets_output() {
        let mut panel = OutputPanel::default();
        panel.stdout.push("out".into());
        panel.stderr.push("err".into());
        panel.stdin_history.push("in".into());
        panel.stdin_cursor = 1;
        panel.stdin_text = "typing".into();
        panel.active_tab = OutputTab::Stderr;
        panel.clear_for_run();
        assert!(panel.stdout.is_empty());
        assert!(panel.stderr.is_empty());
        // stdin_history is preserved (allows pre-typing input)
        assert_eq!(panel.stdin_history.len(), 1);
        assert_eq!(panel.stdin_cursor, 0); // cursor reset
        assert!(panel.stdin_text.is_empty());
        assert_eq!(panel.active_tab, OutputTab::Stdout);
    }

    #[test]
    fn select_tab() {
        let mut panel = OutputPanel::default();
        panel.update(&OutputMessage::SelectTab(OutputTab::Stderr));
        assert_eq!(panel.active_tab, OutputTab::Stderr);
        panel.update(&OutputMessage::SelectTab(OutputTab::Stdin));
        assert_eq!(panel.active_tab, OutputTab::Stdin);
        panel.update(&OutputMessage::SelectTab(OutputTab::Stdout));
        assert_eq!(panel.active_tab, OutputTab::Stdout);
    }

    #[test]
    fn close_hides_panel() {
        let mut panel = OutputPanel::default();
        panel.visible = true;
        panel.update(&OutputMessage::Close);
        assert!(!panel.visible);
    }

    #[test]
    fn stdin_input_updates_text() {
        let mut panel = OutputPanel::default();
        let result = panel.update(&OutputMessage::StdinInput("hello".into()));
        assert!(result.is_none());
        assert_eq!(panel.stdin_text, "hello");
    }

    #[test]
    fn stdin_submit_returns_line() {
        let mut panel = OutputPanel::default();
        panel.stdin_text = "hello world".into();
        let result = panel.update(&OutputMessage::StdinSubmit);
        assert_eq!(result, Some("hello world".into()));
        assert!(panel.stdin_text.is_empty());
        assert_eq!(panel.stdin_history, vec!["hello world"]);
    }

    #[test]
    fn stdin_submit_empty_returns_none() {
        let mut panel = OutputPanel::default();
        let result = panel.update(&OutputMessage::StdinSubmit);
        assert!(result.is_none());
    }

    #[test]
    fn view_renders_without_panic() {
        let panel = OutputPanel::default();
        let _element: Element<'_, OutputMessage> = panel.view();
    }

    #[test]
    fn view_stdout_tab_renders() {
        let mut panel = OutputPanel::default();
        panel.stdout.push("line 1".into());
        let _element: Element<'_, OutputMessage> = panel.view();
    }

    #[test]
    fn view_stderr_tab_renders() {
        let mut panel = OutputPanel::default();
        panel.active_tab = OutputTab::Stderr;
        panel.stderr.push("error line".into());
        let _element: Element<'_, OutputMessage> = panel.view();
    }

    #[test]
    fn view_stdin_tab_not_running() {
        let mut panel = OutputPanel::default();
        panel.active_tab = OutputTab::Stdin;
        let _element: Element<'_, OutputMessage> = panel.view();
    }

    #[test]
    fn view_stdin_tab_running() {
        let mut panel = OutputPanel::default();
        panel.active_tab = OutputTab::Stdin;
        panel.running = true;
        let _element: Element<'_, OutputMessage> = panel.view();
    }

    #[test]
    fn view_stdin_tab_with_history() {
        let mut panel = OutputPanel::default();
        panel.active_tab = OutputTab::Stdin;
        panel.running = true;
        panel.stdin_history.push("first line".into());
        panel.stdin_history.push("second line".into());
        let _element: Element<'_, OutputMessage> = panel.view();
    }
}

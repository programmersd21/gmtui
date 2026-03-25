use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use gmtui::app::App;
use gmtui::config::{AuthConfig, Config};
use gmtui::gmail::{GmailAuth, GmailClient};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let config = Config::load()?;
    let auth_config = AuthConfig::load()?;

    let mut auth = GmailAuth::new(&auth_config)?;
    auth.authenticate()?;

    let client = GmailClient::new(auth);
    let mut app = App::new(config, client);

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let cleanup = || {
        let mut stdout = io::stdout();
        let _ = disable_raw_mode();
        let _ = execute!(stdout, LeaveAlternateScreen);
    };

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        cleanup();
        default_hook(info);
    }));

    let result = app.run(&mut terminal).await;
    cleanup();
    result
}

use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct TrayManager {
    _tray_icon: TrayIcon,
    settings_id: String,
    exit_id: String,
}

pub enum TrayAction {
    None,
    OpenSettings,
    Exit,
}

impl TrayManager {
    pub fn new() -> Self {
        let menu = Menu::new();
        let settings_item = MenuItem::new("Settings", true, None);
        let settings_id = settings_item.id().0.clone();

        let exit_item = MenuItem::new("Exit", true, None);
        let exit_id = exit_item.id().0.clone();

        let _ = menu.append_items(&[&settings_item, &PredefinedMenuItem::separator(), &exit_item]);

        let icon = tray_icon::Icon::from_rgba(vec![128; 32 * 32 * 4], 32, 32).unwrap();

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Gescher")
            .with_icon(icon)
            .build()
            .unwrap();

        Self {
            _tray_icon: tray_icon,
            settings_id,
            exit_id,
        }
    }

    pub fn update(&self) -> TrayAction {
        // トレイ（アイコン）自体のイベントを確実に拾って空にする
        while let Ok(_) = TrayIconEvent::receiver().try_recv() {}

        // メニューイベントの処理
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id.0 == self.settings_id {
                return TrayAction::OpenSettings;
            } else if event.id.0 == self.exit_id {
                return TrayAction::Exit;
            }
        }

        TrayAction::None
    }
}

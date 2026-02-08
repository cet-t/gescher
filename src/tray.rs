use crate::app_state;
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct TrayManager {
    _tray_icon: TrayIcon,
    toggle_item: CheckMenuItem,
    quit_item: MenuItem,
}

impl TrayManager {
    pub fn new() -> Self {
        let menu = Menu::new();
        let toggle_item = CheckMenuItem::new("有効", true, true, None);
        let quit_item = MenuItem::new("終了", true, None);

        menu.append_items(&[&toggle_item, &PredefinedMenuItem::separator(), &quit_item])
            .unwrap();

        let icon = Self::load_icon();

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(true)
            .with_tooltip("Gescher Mouse Gesture")
            .with_icon(icon)
            .build()
            .unwrap();

        Self {
            _tray_icon: tray_icon,
            toggle_item,
            quit_item,
        }
    }

    fn load_icon() -> Icon {
        let width = 16;
        let height = 16;
        let rgba = vec![64, 64, 64, 255].repeat(width * height);
        Icon::from_rgba(rgba, width as u32, height as u32).expect("Failed to create icon")
    }

    pub fn update(&self) -> bool {
        #[cfg(target_os = "windows")]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
            };
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        while let Ok(_event) = TrayIconEvent::receiver().try_recv() {}

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.toggle_item.id() {
                let active = self.toggle_item.is_checked();
                app_state::set_active(active);
                println!("App Activity Toggled: {}", active);
            } else if event.id == self.quit_item.id() {
                return true;
            }
        }
        false
    }
}

//! ## Example
//! [Open full example with winit here 🢅](https://github.com/Ciantic/trayicon-rs/blob/master/examples/winit/src/main.rs)

use std::fmt::Debug;

#[cfg(target_os = "windows")]
#[path = "./sys/windows/mod.rs"]
mod sys;

/// Tray Icon event sender
#[derive(Debug, Clone)]
pub(crate) enum TrayIconSender<T>
where
    T: PartialEq + Clone + 'static,
{
    Std(std::sync::mpsc::Sender<T>),

    #[cfg(feature = "winit")]
    Winit(winit::event_loop::EventLoopProxy<T>),

    #[cfg(feature = "crossbeam-channel")]
    Crossbeam(crossbeam_channel::Sender<T>),
}

impl<T> TrayIconSender<T>
where
    T: PartialEq + Clone + 'static,
{
    pub fn send(&self, e: &T) {
        match self {
            TrayIconSender::Std(s) => {
                let _ = s.send(e.clone());
            }
            #[cfg(feature = "winit")]
            TrayIconSender::Winit(s) => {
                let _ = s.send_event(e.clone());
            }
            #[cfg(feature = "crossbeam-channel")]
            TrayIconSender::Crossbeam(s) => {
                let _ = s.try_send(e.clone());
            }
        }
    }
}

#[derive(Clone)]
pub struct Icon {
    buffer: Option<&'static [u8]>,
    sys: sys::IconSys,
}

impl Debug for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Icon")
    }
}

impl Icon {
    pub fn from_buffer(
        buffer: &'static [u8],
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<Icon, Error> {
        Ok(Icon {
            buffer: Some(buffer),
            sys: sys::IconSys::from_buffer(buffer, width, height)?,
        })
    }
}

impl PartialEq for Icon {
    fn eq(&self, other: &Self) -> bool {
        self.buffer == other.buffer
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuItem<T>
where
    T: PartialEq + Clone + 'static,
{
    Separator,
    Item {
        name: String,
        event: T,
        disabled: bool,
        icon: Option<Icon>,
    },
    CheckableItem {
        name: String,
        is_checked: bool,
        event: T,
        disabled: bool,
        icon: Option<Icon>,
    },
    ChildMenu {
        name: String,
        children: MenuBuilder<T>,
        disabled: bool,
        icon: Option<Icon>,
    },
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MenuBuilder<T>
where
    T: PartialEq + Clone + 'static,
{
    menu_items: Vec<MenuItem<T>>,
}

/// Menu Builder
///
/// This is defined as consuming builder, could be converted to non-consuming
/// one. This builder includes conditional helper `when` for composing
/// conditionally some items.
impl<T> MenuBuilder<T>
where
    T: PartialEq + Clone + 'static,
{
    pub fn new() -> MenuBuilder<T> {
        MenuBuilder { menu_items: vec![] }
    }

    /// Conditionally include items, poor mans function composition
    pub fn when<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }

    pub fn with(mut self, item: MenuItem<T>) -> Self {
        self.menu_items.push(item);
        self
    }

    pub fn separator(mut self) -> Self {
        self.menu_items.push(MenuItem::Separator);
        self
    }

    pub fn item(mut self, name: &str, on_click: T) -> Self {
        self.menu_items.push(MenuItem::Item {
            name: name.to_string(),
            event: on_click,
            disabled: false,
            icon: None,
        });
        self
    }

    pub fn checkable(mut self, name: &str, is_checked: bool, on_click: T) -> Self {
        self.menu_items.push(MenuItem::CheckableItem {
            name: name.to_string(),
            is_checked,
            event: on_click,
            disabled: false,
            icon: None,
        });
        self
    }

    pub fn child_menu(mut self, name: &str, menu: MenuBuilder<T>) -> Self {
        self.menu_items.push(MenuItem::ChildMenu {
            name: name.to_string(),
            children: menu,
            disabled: false,
            icon: None,
        });
        self
    }

    pub(crate) fn build(&self) -> Result<crate::sys::MenuSys<T>, Error> {
        sys::build_menu(self)
    }
}

/// Tray Icon builder
///
/// Start by choosing an event sender implementation. There are three different
/// senders depending on the optional features. By default the sender function
/// uses `std::sync::mpsc::Sender<T>`, additionally if `winit` feature is
/// enabled you can choose to use `winit::event_loop::EventLoopProxy<T>` or with
/// `crossbeam` feature the `crossbeam_channel::Sender<T>` is available.
///
/// This is defined as consuming builder, this includes conditional helper
/// `when` for composing conditionally some settings.
///
/// [Open full example with winit here 🢅](https://github.com/Ciantic/trayicon-rs/blob/master/examples/winit/src/main.rs)
#[derive(Debug, Clone)]
pub struct TrayIconBuilder<T>
where
    T: PartialEq + Clone + 'static,
{
    icon: Result<Icon, Error>,
    width: Option<u32>,
    height: Option<u32>,
    menu: Option<MenuBuilder<T>>,
    on_click: Option<T>,
    on_double_click: Option<T>,
    on_right_click: Option<T>,
    sender: Option<TrayIconSender<T>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    IconLoadingFailed,
    SenderMissing,
    IconMissing,
    OsError,
}

impl From<&Error> for Error {
    fn from(e: &Error) -> Self {
        *e
    }
}

impl<T> TrayIconBuilder<T>
where
    T: PartialEq + Clone + 'static,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> TrayIconBuilder<T> {
        TrayIconBuilder {
            icon: Err(Error::IconMissing),
            width: None,
            height: None,
            menu: None,
            on_click: None,
            on_double_click: None,
            on_right_click: None,
            sender: None,
        }
    }

    /// Conditionally include items, poor mans function composition
    pub fn when<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }

    pub fn sender(mut self, s: std::sync::mpsc::Sender<T>) -> Self {
        self.sender = Some(TrayIconSender::Std(s));
        self
    }

    /// Optional feature, requires `winit` feature
    #[cfg(feature = "winit")]
    pub fn sender_winit(mut self, s: winit::event_loop::EventLoopProxy<T>) -> Self {
        self.sender = Some(TrayIconSender::Winit(s));
        self
    }

    /// Optional feature, requires `crossbeam-channel` feature
    #[cfg(feature = "crossbeam-channel")]
    pub fn sender_crossbeam(mut self, s: crossbeam_channel::Sender<T>) -> Self {
        self.sender = Some(TrayIconSender::Crossbeam(s));
        self
    }

    pub fn on_click(mut self, event: T) -> Self {
        self.on_click = Some(event);
        self
    }

    pub fn on_double_click(mut self, event: T) -> Self {
        self.on_double_click = Some(event);
        self
    }

    pub fn on_right_click(mut self, event: T) -> Self {
        self.on_right_click = Some(event);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Ok(icon);
        self
    }

    pub fn icon_from_buffer(mut self, buffer: &'static [u8]) -> Self {
        self.icon = Icon::from_buffer(buffer, None, None);
        self
    }

    pub fn menu(mut self, menu: MenuBuilder<T>) -> Self
    where
        T: PartialEq + Clone + 'static,
    {
        self.menu = Some(menu);
        self
    }

    pub fn build(self) -> Result<TrayIcon<T>, Error> {
        Ok(TrayIcon::new(crate::sys::build_trayicon(&self)?, self))
    }
}

pub struct TrayIcon<T>
where
    T: PartialEq + Clone + 'static,
{
    sys: Box<crate::sys::TrayIconSys<T>>,
    builder: TrayIconBuilder<T>,
}

impl<T> TrayIcon<T>
where
    T: PartialEq + Clone + 'static,
{
    pub(crate) fn new(
        sys: Box<crate::sys::TrayIconSys<T>>,
        builder: TrayIconBuilder<T>,
    ) -> TrayIcon<T> {
        TrayIcon { builder, sys }
    }

    // Set the icon if changed
    pub fn set_icon(&mut self, icon: &Icon) -> Result<(), Error> {
        if let Ok(old_icon) = &self.builder.icon {
            if old_icon == icon {
                return Ok(());
            }
        }
        self.builder.icon = Ok(icon.clone());
        self.sys.set_icon(&icon)
    }

    /// Set the menu if changed
    pub fn set_menu(&mut self, menu: &MenuBuilder<T>) -> Result<(), Error> {
        if let Some(old_menu) = &self.builder.menu {
            if old_menu == menu {
                return Ok(());
            }
        }
        self.builder.menu = Some(menu.clone());
        self.sys.set_menu(&menu)
    }
}

unsafe impl<T> Sync for TrayIcon<T> where T: PartialEq + Clone + 'static {}

unsafe impl<T> Send for TrayIcon<T> where T: PartialEq + Clone + 'static {}

/// This is just helper for Sys packages, not an enforcement through generics
pub(crate) trait TrayIconBase<T>
where
    T: PartialEq + Clone + 'static,
{
    fn set_icon(&mut self, icon: &Icon) -> Result<(), Error>;
    fn set_menu(&mut self, menu: &MenuBuilder<T>) -> Result<(), Error>;
}

use vizia_plug::vizia::prelude::*;

pub struct ColorPalette;

impl ColorPalette {
    pub const BACKGROUND: Color = Color::rgb(18, 18, 22);
    pub const SURFACE: Color = Color::rgb(24, 24, 28);
    pub const SURFACE_ELEVATED: Color = Color::rgb(32, 32, 38);

    pub const PRIMARY: Color = Color::rgb(99, 102, 241);
    pub const PRIMARY_HOVER: Color = Color::rgb(79, 82, 221);
    pub const PRIMARY_LIGHT: Color = Color::rgb(165, 180, 252);

    pub const OSC1_ACCENT: Color = Color::rgb(59, 130, 246);
    pub const OSC2_ACCENT: Color = Color::rgb(34, 197, 94);
    pub const OSC3_ACCENT: Color = Color::rgb(239, 68, 68);

    pub const TEXT_PRIMARY: Color = Color::rgb(248, 250, 252);
    pub const TEXT_SECONDARY: Color = Color::rgb(148, 163, 184);
    pub const TEXT_MUTED: Color = Color::rgb(100, 116, 139);

    pub const BORDER: Color = Color::rgb(51, 65, 85);
    pub const HOVER: Color = Color::rgb(30, 41, 59);
    pub const ACTIVE: Color = Color::rgb(15, 23, 42);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabId {
    Oscillators,
    Envelope,
}

#[derive(Lens, Clone, Data)]
pub struct TabSwitcherData {
    pub active_tab: i32,
}

impl TabId {
    fn to_index(self) -> i32 {
        match self {
            TabId::Oscillators => 0,
            TabId::Envelope => 1,
        }
    }

    fn from_index(index: i32) -> Self {
        match index {
            0 => TabId::Oscillators,
            1 => TabId::Envelope,
            _ => TabId::Oscillators,
        }
    }
}

pub enum TabSwitcherEvent {
    SetActiveTab(TabId),
}

impl Model for TabSwitcherData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|tab_event, _| match tab_event {
            TabSwitcherEvent::SetActiveTab(tab_id) => {
                self.active_tab = tab_id.to_index();
            }
        });
    }
}

impl TabSwitcherData {
    pub fn new() -> Self {
        Self {
            active_tab: TabId::Oscillators.to_index(),
        }
    }
}

pub struct TabSwitcher;

impl TabSwitcher {
    pub fn new<F>(cx: &mut Context, content_builder: F) -> Handle<impl View>
    where
        F: Fn(&mut Context, TabId) + 'static,
    {
        TabSwitcherData::new().build(cx);

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Self::tab_button(cx, "Oscillators", TabId::Oscillators);
                Self::tab_button(cx, "Envelope", TabId::Envelope);
            })
            .height(Pixels(40.0))
            .background_color(ColorPalette::SURFACE)
            .border_width(Pixels(1.0))
            .border_color(ColorPalette::BORDER);

            Binding::new(
                cx,
                TabSwitcherData::active_tab,
                move |cx, active_tab_index| {
                    let tab_id = TabId::from_index(active_tab_index.get(cx));
                    content_builder(cx, tab_id);
                },
            );
        })
    }

    fn tab_button<'a>(cx: &'a mut Context, label: &str, tab_id: TabId) -> Handle<'a, impl View> {
        Button::new(cx, |cx| {
            Label::new(cx, label)
                .font_size(12.0)
                .color(ColorPalette::TEXT_PRIMARY)
        })
        .height(Stretch(1.0))
        .width(Pixels(120.0))
        .background_color(TabSwitcherData::active_tab.map(move |active_index| {
            if TabId::from_index(*active_index) == tab_id {
                ColorPalette::PRIMARY
            } else {
                Color::transparent()
            }
        }))
        .border_width(Pixels(0.0))
        .corner_radius(Pixels(0.0))
        .cursor(CursorIcon::Hand)
        .on_press(move |cx| {
            cx.emit(TabSwitcherEvent::SetActiveTab(tab_id));
        })
    }
}

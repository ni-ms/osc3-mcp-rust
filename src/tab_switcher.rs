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

#[derive(Clone, Debug, Data, PartialEq)]
pub struct TabDefinition {
    pub id: String,
    pub label: String,
    pub width: Option<f32>,
}

impl TabDefinition {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            width: None,
        }
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

#[derive(Lens, Clone, Data)]
pub struct TabSwitcherData {
    pub active_tab_id: String,
    pub tabs: Vec<TabDefinition>,
}

pub enum TabSwitcherEvent {
    SetActiveTab(String),
    SetTabs(Vec<TabDefinition>),
}

impl Model for TabSwitcherData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|tab_event, _| match tab_event {
            TabSwitcherEvent::SetActiveTab(tab_id) => {
                if self.tabs.iter().any(|tab| tab.id == *tab_id) {
                    self.active_tab_id = tab_id.clone();
                }
            }
            TabSwitcherEvent::SetTabs(tabs) => {
                self.tabs = tabs.clone();

                if !tabs.iter().any(|tab| tab.id == self.active_tab_id) {
                    if let Some(first_tab) = tabs.first() {
                        self.active_tab_id = first_tab.id.clone();
                    }
                }
            }
        });
    }
}

impl TabSwitcherData {
    pub fn new(tabs: Vec<TabDefinition>) -> Self {
        let active_tab_id = tabs.first().map(|tab| tab.id.clone()).unwrap_or_default();

        Self {
            active_tab_id,
            tabs,
        }
    }

    pub fn get_active_tab_index(&self) -> usize {
        self.tabs
            .iter()
            .position(|tab| tab.id == self.active_tab_id)
            .unwrap_or(0)
    }
}

pub struct TabSwitcher;

impl TabSwitcher {
    pub fn new<F>(
        cx: &mut Context,
        tabs: Vec<TabDefinition>,
        content_builder: F,
    ) -> Handle<impl View>
    where
        F: 'static + Fn(&mut Context, &str, usize),
    {
        TabSwitcherData::new(tabs).build(cx);

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                let data = cx.data::<TabSwitcherData>().unwrap();
                let tabs_clone = data.tabs.clone();

                for tab in tabs_clone.iter() {
                    Self::tab_button(cx, tab.clone());
                }
            })
            .height(Pixels(40.0))
            .background_color(ColorPalette::SURFACE)
            .border_width(Pixels(1.0))
            .border_color(ColorPalette::BORDER);

            Binding::new(
                cx,
                TabSwitcherData::active_tab_id,
                move |cx, active_tab_id| {
                    let data = cx.data::<TabSwitcherData>().unwrap();
                    let active_index = data.get_active_tab_index();
                    let active_id = active_tab_id.get(cx);
                    content_builder(cx, &*active_id, active_index);
                },
            );
        })
    }

    pub fn new_dynamic<F>(
        cx: &mut Context,
        tabs: Vec<TabDefinition>,
        content_builder: F,
    ) -> Handle<impl View>
    where
        F: 'static + Fn(&mut Context, &str, usize),
    {
        TabSwitcherData::new(tabs).build(cx);

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Binding::new(cx, TabSwitcherData::tabs, |cx, tabs_lens| {
                    let tabs = tabs_lens.get(cx);
                    for tab in tabs.iter() {
                        Self::tab_button(cx, tab.clone());
                    }
                });
            })
            .height(Pixels(40.0))
            .background_color(ColorPalette::SURFACE)
            .border_width(Pixels(1.0))
            .border_color(ColorPalette::BORDER);

            Binding::new(
                cx,
                TabSwitcherData::active_tab_id,
                move |cx, active_tab_id| {
                    let data = cx.data::<TabSwitcherData>().unwrap();
                    let active_index = data.get_active_tab_index();
                    let active_id = active_tab_id.get(cx);
                    content_builder(cx, &*active_id, active_index);
                },
            );
        })
    }

    pub fn new_indexed<F>(
        cx: &mut Context,
        tabs: Vec<TabDefinition>,
        content_builder: F,
    ) -> Handle<impl View>
    where
        F: 'static + Fn(&mut Context, usize),
    {
        Self::new(cx, tabs, move |cx, _tab_id, index| {
            content_builder(cx, index);
        })
    }

    fn tab_button(cx: &mut Context, tab: TabDefinition) -> Handle<impl View> {
        let tab_id = tab.id.clone();
        let tab_id_for_press = tab.id.clone();
        let tab_width = tab.width.unwrap_or(120.0);

        Button::new(cx, |cx| {
            Label::new(cx, &tab.label)
                .font_size(12.0)
                .color(ColorPalette::TEXT_PRIMARY)
        })
        .height(Stretch(1.0))
        .width(Pixels(tab_width))
        .background_color(TabSwitcherData::active_tab_id.map(move |active_id| {
            if *active_id == tab_id {
                ColorPalette::PRIMARY
            } else {
                Color::transparent()
            }
        }))
        .border_width(Pixels(0.0))
        .corner_radius(Pixels(0.0))
        .cursor(CursorIcon::Hand)
        .on_press(move |cx| {
            cx.emit(TabSwitcherEvent::SetActiveTab(tab_id_for_press.clone()));
        })
    }
}
macro_rules! tabs {
    ($($id:expr => $label:expr),* $(,)?) => {
        vec![
            $(TabDefinition::new($id, $label)),*
        ]
    };
}

macro_rules! tabs_with_width {
    ($($id:expr => $label:expr => ($width:expr)),* $(,)?) => {
        vec![
            $(TabDefinition::new($id, $label).with_width($width)),*
        ]
    };
}

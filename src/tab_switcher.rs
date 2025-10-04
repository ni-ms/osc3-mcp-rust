use vizia_plug::vizia::prelude::*;

const TABSWITCHER_THEME: &str = r#"
.tabbar {
  background-color: #18181E;
  border-bottom: 1px solid #334155;
}

.tabbar-inner {
  child-space: 0px;
  col-between: 0px;
}

button.tab {
  background-color: transparent;
  color: #F8FAFC;
  border-width: 0px;
  padding-left: 12px;
  padding-right: 12px;
  height: 100%;
  font-size: 12px;
  transition: background-color 120ms ease, color 120ms ease;
}

button.tab:hover {
  background-color: #1E293B;
}

button.tab.active {
  background-color: #6366F1;
  color: #0B1020;
}

button.tab .tab-label {
  text-shadow: 0px 0px 0px rgba(0,0,0,0.0);
}

"#;

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
                if self.tabs.iter().any(|t| t.id == *tab_id) {
                    self.active_tab_id = tab_id.clone();
                }
            }
            TabSwitcherEvent::SetTabs(tabs) => {
                self.tabs = tabs.clone();
                if !self.tabs.iter().any(|t| t.id == self.active_tab_id) {
                    if let Some(first) = self.tabs.first() {
                        self.active_tab_id = first.id.clone();
                    }
                }
            }
        });
    }
}

impl TabSwitcherData {
    pub fn new(tabs: Vec<TabDefinition>) -> Self {
        let active_tab_id = tabs.first().map(|t| t.id.clone()).unwrap_or_default();
        Self {
            active_tab_id,
            tabs,
        }
    }

    pub fn get_active_tab_index(&self) -> usize {
        self.tabs
            .iter()
            .position(|t| t.id == self.active_tab_id)
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
        cx.add_stylesheet(TABSWITCHER_THEME);

        TabSwitcherData::new(tabs).build(cx);

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Binding::new(cx, TabSwitcherData::active_tab_id, |cx, active_lens| {
                    let active_id = active_lens.get(cx).clone();
                    Binding::new(cx, TabSwitcherData::tabs, move |cx, tabs_lens| {
                        let tabs_vec = tabs_lens.get(cx).clone();

                        HStack::new(cx, |cx| {
                            for tab in tabs_vec.iter() {
                                let is_active = tab.id == active_id;
                                Self::tab_button(cx, tab.clone(), is_active);
                            }
                        })
                        .class("tabbar-inner");
                    });
                });
            })
            .height(Pixels(40.0))
            .class("tabbar");

            Binding::new(
                cx,
                TabSwitcherData::active_tab_id,
                move |cx, active_tab_id| {
                    let data = cx.data::<TabSwitcherData>().unwrap();
                    let active_index = data.get_active_tab_index();
                    let active_id = active_tab_id.get(cx);
                    VStack::new(cx, |cx| {
                        content_builder(cx, &*active_id, active_index);
                    })
                    .class("tabcontent");
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
            content_builder(cx, index)
        })
    }

    fn tab_button(cx: &mut Context, tab: TabDefinition, is_active: bool) -> Handle<impl View> {
        let tab_id_for_press = tab.id.clone();
        let width = tab.width.unwrap_or(120.0);

        let mut handle = Button::new(cx, |cx| Label::new(cx, &tab.label).class("tab-label"))
            .class("tab")
            .width(Pixels(width))
            .height(Stretch(1.0))
            .cursor(CursorIcon::Hand)
            .on_press(move |cx| cx.emit(TabSwitcherEvent::SetActiveTab(tab_id_for_press.clone())));

        if is_active {
            handle = handle.class("active");
        }

        handle
    }
}

macro_rules! tabs {
    ($($id:expr => $label:expr),* $(,)?) => {
        vec![$(TabDefinition::new($id, $label)),*]
    };
}

macro_rules! tabs_with_width {
    ($($id:expr => $label:expr => ($width:expr)),* $(,)?) => {
        vec![$(TabDefinition::new($id, $label).with_width($width)),*]
    };
}

use vizia_plug::vizia::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Data)]
pub enum TabId {
    Oscillators,
    Envelope,
}

#[derive(Lens, Clone, Data)]
pub struct TabSwitcherData {
    pub active_tab: TabId,
}

pub enum TabSwitcherEvent {
    SetActiveTab(TabId),
}

impl Model for TabSwitcherData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|tab_event, _| match tab_event {
            TabSwitcherEvent::SetActiveTab(tab_id) => {
                self.active_tab = *tab_id;
            }
        });
    }
}

impl TabSwitcherData {
    pub fn new() -> Self {
        Self {
            active_tab: TabId::Oscillators,
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
            // Tab bar
            HStack::new(cx, |cx| {
                Self::tab_button(cx, "Oscillators", TabId::Oscillators);
                Self::tab_button(cx, "Envelope", TabId::Envelope);
            })
            .height(Pixels(40.0))
            .background_color(Color::rgb(24, 24, 28))
            .border_width(Pixels(1.0))
            .border_color(Color::rgb(51, 65, 85));

            // Content area
            Binding::new(cx, TabSwitcherData::active_tab, move |cx, active_tab| {
                let tab_id = active_tab.get(cx);
                content_builder(cx, tab_id);
            });
        })
    }

    fn tab_button<'a>(cx: &'a mut Context, label: &str, tab_id: TabId) -> Handle<'a, impl View> {
        Button::new(cx, |cx| {
            Label::new(cx, label)
                .font_size(12.0)
                .color(Color::rgb(248, 250, 252))
        })
        .height(Stretch(1.0))
        .width(Pixels(120.0))
        .background_color(TabSwitcherData::active_tab.map(move |active| {
            if *active == tab_id {
                Color::rgb(99, 102, 241) // Active tab color
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

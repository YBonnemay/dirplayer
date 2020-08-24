use util::{SinSignal, StatefulList, TabsState};
// use self::util::{RandomSignal, SinSignal, StatefulList, TabsState};
// use crate::util::{RandomSignal, SinSignal, StatefulList, TabsState};

const TASKS: [&str; 24] = [
    "Item1", "Item2", "Item3", "Item4", "Item5", "Item6", "Item7", "Item8", "Item9", "Item10",
    "Item11", "Item12", "Item13", "Item14", "Item15", "Item16", "Item17", "Item18", "Item19",
    "Item20", "Item21", "Item22", "Item23", "Item24",
];

pub struct Signal<S: Iterator> {
    source: S,
    pub points: Vec<S::Item>,
    tick_rate: usize,
}

impl<S> Signal<S>
where
    S: Iterator,
{
    fn on_tick(&mut self) {
        for _ in 0..self.tick_rate {
            self.points.remove(0);
        }
        self.points
            .extend(self.source.by_ref().take(self.tick_rate));
    }
}

pub struct Signals {
    pub sin1: Signal<SinSignal>,
    pub sin2: Signal<SinSignal>,
    pub window: [f64; 2],
}

impl Signals {
    fn on_tick(&mut self) {
        self.sin1.on_tick();
        self.sin2.on_tick();
        self.window[0] += 1.0;
        self.window[1] += 1.0;
    }
}

pub struct Server<'a> {
    pub name: &'a str,
    pub location: &'a str,
    pub coords: (f64, f64),
    pub status: &'a str,
}

pub struct Data<'a> {
    pub should_quit: bool,
    pub tabs: TabsState<'a>,
    pub tasks: StatefulList<&'a str>,
    pub show_chart: bool,
    // pub progress: f64,
    // pub sparkline: Signal<RandomSignal>,
    // pub tasks: StatefulList<&'a str>,
    // pub logs: StatefulList<(&'a str, &'a str)>,
    // pub signals: Signals,
    // pub barchart: Vec<(&'a str, u64)>,
    // pub servers: Vec<Server>,
    // pub enhanced_graphics: bool,
}

impl<'a> Data<'a> {
    pub fn new() -> Data<'a> {
        Data {
            should_quit: false,
            tabs: TabsState::new(vec!["Tab0", "Tab1"]),
            tasks: StatefulList::with_items(TASKS.to_vec()),
            show_chart: true,
        }
    }

    pub fn on_up(&mut self) {}
    pub fn on_down(&mut self) {}
    pub fn on_right(&mut self) {}
    pub fn on_left(&mut self) {}
    pub fn on_key(&mut self, _c: char) {}
    pub fn on_tick(&mut self) {}
}

trait Key: Clone {
    fn handle_event(&self);
    fn change(&mut self, value: &str);
    fn default() -> Self;
}

#[derive(Clone)]
struct A {
    value: String,
}

impl Key for A {
    fn handle_event(&self) {
        println!("Handling event in A: {}", self.value);
    }

    fn change(&mut self, value: &str) {
        self.value = value.to_string();
    }

    fn default() -> Self {
        A {
            value: "Alpha".to_string(),
        }
    }
}

#[derive(Clone)]
struct B {
    value: String,
}

impl Key for B {
    fn handle_event(&self) {
        println!("Handling event in B: {}", self.value);
    }

    fn change(&mut self, value: &str) {
        self.value = value.to_string();
    }
    fn default() -> Self {
        B {
            value: "Beta".to_string(),
        }
    }
}

#[derive(Clone)]
enum Pages<T: Key> {
    A(T),
    B(T),
}

impl<T: Key> Default for Pages<T> {
    fn default() -> Self {
        // Default to variant A with a default instance of T
        Pages::A(T::default())
    }
}

impl<T: Key> Key for Pages<T> {
    fn handle_event(&self) {
        match self {
            Pages::A(page) => page.handle_event(),
            Pages::B(page) => page.handle_event(),
        }
    }

    fn change(&mut self, value: &str) {
        match self {
            Pages::A(page) => page.change(value),
            Pages::B(page) => page.change(value),
        }
    }

    fn default() -> Self {
        Pages::A(T::default())
    }
}

struct App<S: Key, T: Key> {
    pub active_page: Pages<S>,
    pub state: SetupState<T>,
}

impl<S: Key, T: Key> App<S, T> {
    pub fn get_state(&self) -> SetupState<T> {
        match self.active_page {
            Pages::A(_) => self.state.clone(),
            Pages::B(_) => self.state.clone(),
        }
    }
}

#[derive(Clone)]
struct SetupState<T: Key> {
    pub active: T,
}

pub fn main() {
    let a = A::default();
    let b = B::default();

    let mut app = App {
        state: SetupState { active: a.clone() },
        active_page: Pages::B(b),
    };

    app.active_page.handle_event();
    app.active_page.change("Delta");
    app.active_page.handle_event();
}

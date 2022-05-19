use uuid::Uuid;
use yew::prelude::*;
use yew_router::prelude::*;

mod home;
mod servers;
mod players;
mod player;
mod server;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/servers")]
    Servers,
    #[at("/server/{uuid}")]
    Server(Uuid),
    #[at("/players")]
    Players,
    #[at("/players/{uuid}")]
    Player(Uuid),
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: &Route) -> Html {

    match routes {
        Route::Home => html! { <h1>{ "Home" }</h1> },
        Route::NotFound => html! { <h1>{ "Erreur 404" }</h1> },
        Route::Servers => html! { <h1>{ "Home" }</h1> },
        Route::Server(_) => html! { <h1>{ "Home" }</h1> },
        Route::Players => html! { <h1>{ "Home" }</h1> },
        Route::Player(_) => html! { <h1>{ "Home" }</h1> },
    }
}

#[function_component(Main)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={Switch::render(switch)} />
        </BrowserRouter>
    }
}

fn main() {
    let handle = yew::start_app::<Main>();
}
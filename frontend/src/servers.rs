use yew::{Callback, Component, ComponentLink, Context, Html};

pub struct Servers {}

impl Component for Servers {
    type Message = ();
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().context::<>(Callback::noop());
        Servers
    }


    fn view(&self, ctx: &Context<Self>) -> Html {
        todo!()
    }

}
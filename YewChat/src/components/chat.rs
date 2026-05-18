use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::services::event_bus::EventBus;
use crate::{services::websocket::WebsocketService, User};

pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
    StartReply(usize),
    CancelReply,
    AddEmoji(&'static str),
}

#[derive(Deserialize)]
struct MessageData {
    from: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

#[derive(Clone)]
struct UserProfile {
    name: String,
    avatar: String,
}

pub struct Chat {
    users: Vec<UserProfile>,
    chat_input: NodeRef,
    _producer: Box<dyn Bridge<EventBus>>,
    wss: WebsocketService,
    messages: Vec<MessageData>,
    reply_to: Option<usize>,
}
impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("context to be set");
        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        let message = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.to_string()),
            data_array: None,
        };

        if let Ok(_) = wss
            .tx
            .clone()
            .try_send(serde_json::to_string(&message).unwrap())
        {
            log::debug!("message sent successfully");
        }

        Self {
            users: vec![],
            messages: vec![],
            reply_to: None,
            chat_input: NodeRef::default(),
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(s) => {
                let msg: WebSocketMessage = serde_json::from_str(&s).unwrap();
                match msg.message_type {
                    MsgTypes::Users => {
                        let users_from_message = msg.data_array.unwrap_or_default();
                        self.users = users_from_message
                            .iter()
                            .map(|u| UserProfile {
                                name: u.into(),
                                avatar: format!(
                                    "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                    u
                                )
                                .into(),
                            })
                            .collect();
                        return true;
                    }
                    MsgTypes::Message => {
                        let message_data: MessageData =
                            serde_json::from_str(&msg.data.unwrap()).unwrap();
                        self.messages.push(message_data);
                        return true;
                    }
                    _ => {
                        return false;
                    }
                }
            }
            Msg::SubmitMessage => {
                let input = self.chat_input.cast::<HtmlInputElement>();
                if let Some(input) = input {
                    let mut outgoing = input.value();
                    if outgoing.trim().is_empty() {
                        return false;
                    }
                    if let Some(reply_index) = self.reply_to {
                        if let Some(reply_msg) = self.messages.get(reply_index) {
                            outgoing =
                                format!("Reply to {}: \"{}\" | {}", reply_msg.from, reply_msg.message, outgoing);
                        }
                    }
                    let message = WebSocketMessage {
                        message_type: MsgTypes::Message,
                        data: Some(outgoing),
                        data_array: None,
                    };
                    if let Err(e) = self
                        .wss
                        .tx
                        .clone()
                        .try_send(serde_json::to_string(&message).unwrap())
                    {
                        log::debug!("error sending to channel: {:?}", e);
                    }
                    input.set_value("");
                    self.reply_to = None;
                };
                false
            }
            Msg::StartReply(index) => {
                self.reply_to = Some(index);
                true
            }
            Msg::CancelReply => {
                self.reply_to = None;
                true
            }
            Msg::AddEmoji(emoji) => {
                if let Some(input) = self.chat_input.cast::<HtmlInputElement>() {
                    let mut value = input.value();
                    value.push_str(emoji);
                    input.set_value(&value);
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);
        let cancel_reply = ctx.link().callback(|_| Msg::CancelReply);
        let add_smile = ctx.link().callback(|_| Msg::AddEmoji("🙂"));
        let add_laugh = ctx.link().callback(|_| Msg::AddEmoji("😂"));
        let add_fire = ctx.link().callback(|_| Msg::AddEmoji("🔥"));
        let add_heart = ctx.link().callback(|_| Msg::AddEmoji("❤️"));

        html! {
            <div class="flex w-screen">
                <div class="flex-none w-56 h-screen bg-gray-100">
                    <div class="text-xl p-3">{"Users"}</div>
                    {
                        self.users.clone().iter().map(|u| {
                            html!{
                                <div class="flex m-3 bg-white rounded-lg p-2">
                                    <div>
                                        <img class="w-12 h-12 rounded-full" src={u.avatar.clone()} alt="avatar"/>
                                    </div>
                                    <div class="flex-grow p-3">
                                        <div class="flex text-xs justify-between">
                                            <div>{u.name.clone()}</div>
                                        </div>
                                        <div class="text-xs text-gray-400">
                                            {"Hi there!"}
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect::<Html>()
                    }
                </div>
                <div class="grow h-screen flex flex-col">
                    <div class="w-full h-14 border-b-2 border-gray-300"><div class="text-xl p-3">{"💬 Chat!"}</div></div>
                    <div class="w-full grow overflow-auto border-b-2 border-gray-300">
                        {
                            self.messages.iter().enumerate().map(|(index, m)| {
                                let user = self.users.iter().find(|u| u.name == m.from).unwrap();
                                let reply = ctx.link().callback(move |_| Msg::StartReply(index));
                                html!{
                                    <div class="flex items-end w-3/6 bg-gray-100 m-8 rounded-tl-lg rounded-tr-lg rounded-br-lg ">
                                        <img class="w-8 h-8 rounded-full m-3" src={user.avatar.clone()} alt="avatar"/>
                                        <div class="p-3 grow">
                                            <div class="text-sm">
                                                {m.from.clone()}
                                            </div>
                                            <div class="text-xs text-gray-500">
                                                if m.message.ends_with(".gif") {
                                                    <img class="mt-3" src={m.message.clone()}/>
                                                } else {
                                                    {m.message.clone()}
                                                }
                                            </div>
                                        </div>
                                        <button onclick={reply} class="mx-3 mb-3 px-2 py-1 text-xs bg-white rounded border border-gray-300 hover:bg-gray-200">
                                            {"Reply"}
                                        </button>
                                    </div>
                                }
                            }).collect::<Html>()
                        }

                    </div>
                    <div class="w-full h-14 flex px-3 items-center">
                        {
                            if let Some(reply_index) = self.reply_to {
                                if let Some(reply_msg) = self.messages.get(reply_index) {
                                    html! {
                                        <div class="absolute bottom-16 left-64 right-6 bg-blue-50 border border-blue-200 rounded px-3 py-2 flex justify-between items-center text-xs">
                                            <div>
                                                {format!("Replying to {}: {}", reply_msg.from, reply_msg.message)}
                                            </div>
                                            <button onclick={cancel_reply} class="ml-3 text-blue-700 underline">
                                                {"Cancel"}
                                            </button>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }
                            } else {
                                html! {}
                            }
                        }
                        <div class="flex gap-1">
                            <button onclick={add_smile} class="px-2 py-1 text-sm bg-gray-100 rounded hover:bg-gray-200">{"🙂"}</button>
                            <button onclick={add_laugh} class="px-2 py-1 text-sm bg-gray-100 rounded hover:bg-gray-200">{"😂"}</button>
                            <button onclick={add_fire} class="px-2 py-1 text-sm bg-gray-100 rounded hover:bg-gray-200">{"🔥"}</button>
                            <button onclick={add_heart} class="px-2 py-1 text-sm bg-gray-100 rounded hover:bg-gray-200">{"❤️"}</button>
                        </div>
                        <input ref={self.chat_input.clone()} type="text" placeholder="Message" class="block w-full py-2 pl-4 mx-3 bg-gray-100 rounded-full outline-none focus:text-gray-700" name="message" required=true />
                        <button onclick={submit} class="p-3 shadow-sm bg-blue-600 w-10 h-10 rounded-full flex justify-center items-center color-white">
                            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" class="fill-white">
                                <path d="M0 0h24v24H0z" fill="none"></path><path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}

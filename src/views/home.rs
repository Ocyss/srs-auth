use crate::api::{self};
use dioxus::{fullstack::Form, prelude::*};

fn player_url(key: &str, play_url: &str) -> String {
    use urlencoding::encode;

    let encoded = encode(play_url);

    let encoded_safe = encoded
        .replace("%3A", ":")
        .replace("%2F", "/")
        .replace("%3F", "?")
        .replace("%26", "&")
        .replace("%3D", "=")
        .replace("%25", "%");

    match key {
        "potplayer" => format!("potplayer:{encoded_safe}"),

        "vlc" => format!("vlc:{encoded_safe}"),

        "iina" => format!("iina:weblink?url={encoded}"),

        "infuse" => format!("infuse:x-callback-url/play?url={encoded}"),

        "mpv" => format!("mpv-handler:{encoded_safe}"),

        "nplayer" => format!("nplayer:{encoded_safe}"),

        "omniplayer" => format!("omniplayer:weblink?url={encoded}"),

        "figplayer" => format!("figplayer:weblink?url={encoded}"),

        "senplayer" => format!("SenPlayer:x-callback-url/play?url={encoded}"),

        "fileball" => format!("filebox:play?url={encoded}"),

        "stellarplayer" => format!("stellar:play/{encoded_safe}"),

        "mxplayer" => format!("mxplayer:{encoded_safe}"),

        "mxplayerpro" => format!("mxplayerpro:{encoded_safe}"),

        "ddplay" => format!("ddplay:{encoded}"),

        _ => "".to_string(),
    }
}

const PLAYERS: &[(&str, &str, &str)] = &[
    ("potplayer", "PotPlayer", "icon-PotPlayer.webp"),
    ("vlc", "VLC", "icon-VLC.webp"),
    ("iina", "IINA", "icon-IINA.webp"),
    ("infuse", "Infuse", "icon-infuse.webp"),
    ("mpv", "MPV", "icon-MPV.webp"),
    ("nplayer", "nPlayer", "icon-NPlayer.webp"),
    ("omniplayer", "OmniPlayer", "icon-OmniPlayer.webp"),
    ("figplayer", "FigPlayer", "icon-FigPlayer.webp"),
    ("senplayer", "SenPlayer", "icon-SenPlayer.webp"),
    ("fileball", "Fileball", "icon-Fileball.webp"),
    ("stellarplayer", "StellarPlayer", "icon-StellarPlayer.webp"),
    ("mxplayer", "MX Player", "icon-MXPlayer.webp"),
    ("mxplayerpro", "MX Player Pro", "icon-MXPlayer.webp"),
    ("ddplay", "弹弹Play", "icon-DDPlay.webp"),
];

#[component]
pub fn Home() -> Element {
    let mut rooms = use_resource(api::rooms::list_rooms);
    let account = use_resource(api::auth::current_account);
    use_future(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(3000).await;
            rooms.restart();
        }
    });
    let mut is_login_open = use_signal(|| false);
    let mut is_create_room_open = use_signal(|| false);
    let mut is_change_password_open = use_signal(|| false);

    let selected_stream_key = use_signal(String::new);

    let room = use_memo(move || {
        let Some(Ok(rooms)) = &*rooms.read() else {
            return Default::default();
        };
        rooms
            .iter()
            .find(|room| room.stream_key == selected_stream_key())
            .or_else(|| rooms.first())
            .cloned()
            .unwrap_or_default()
    });

    let can_manage = use_memo(move || match &*account.read() {
        Some(Ok(account)) => {
            let room = room();
            account.is_administrator || account.username == room.room.owner_username
        }
        _ => false,
    });

    let is_administrator = use_memo(move || match &*account.read() {
        Some(Ok(account)) => account.is_administrator,
        _ => false,
    });

    let publish_info: Resource<anyhow::Result<Option<api::rooms::PublishInfoResponse>>> =
        use_resource(move || async move {
            if !can_manage() || room().stream_key.is_empty() {
                return Ok(None);
            }
            Ok(Some(
                api::rooms::publish_info(room().stream_key.clone()).await?,
            ))
        });

    let resource_errors = use_memo(move || {
        [
            rooms
                .read()
                .as_ref()
                .and_then(|result| result.as_ref().err())
                .map(|error| format!("直播间列表: {error}")),
            account
                .read()
                .as_ref()
                .and_then(|result| result.as_ref().err())
                .filter(|error| !error.to_string().contains("Not authorized"))
                .map(|error| format!("登录状态: {error}")),
            publish_info
                .read()
                .as_ref()
                .and_then(|result| result.as_ref().err())
                .map(|error| format!("推流信息: {error}")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    });

    let mut lock_override = use_signal(|| None::<bool>);
    let mut title_draft = use_signal(String::new);
    let mut play_mode = use_signal(|| "srt");

    let play_url = use_memo(move || {
        let room = room();
        let play_url = &room.play_url;
        play_url
            .as_ref()
            .and_then(|urls| match play_mode() {
                "srt" => Some(&urls.0),
                "rtmp" => Some(&urls.1),
                "http-flv" => Some(&urls.2),
                _ => None,
            })
            .map(|s| s.to_string())
    });

    let mut update_room_action = use_action(api::rooms::update_room);

    let mut logout_action = use_action(|| async move {
        api::auth::logout().await?;
        let _ = document::eval("window.location.reload()");
        Ok::<_, anyhow::Error>(())
    });

    use_effect(move || {
        if let Some((_, _, play_url, _)) = room().play_url {
            document::eval(&format!(
                r#"
                if (typeof window.my_player !== 'undefined') {{
                    window.my_player.switchURL('{play_url}');
                }} else {{
                    window.Player.I18N.use(window.Player.I18N.lang.zh)
                    window.my_player = new window.Player({{
                        lang: 'zh',
                        id: 'player_id',
                        height: '100%',
                        width: '100%',
                        isLive: true,
                        url: '{play_url}',
                        playsinline: true,
                        autoplay: false,
                        plugins: [window.FlvPlayer]
                    }})
                }}
                "#
            ));
            // document::eval(&format!(
            //     r#"
            //     if (typeof window.my_player !== 'undefined') {{
            //         window.my_player.load({{
            //             sources: [
            //                     {{
            //                         label: 'label_for_webrtc',
            //                         type: 'webrtc',
            //                         file: '{play_url}'
            //                     }}
            //             ]
            //         }});
            //     }} else {{
            //         window.my_player = OvenPlayer.create('player_id', {{
            //             sources: [
            //                     {{
            //                         label: 'label_for_webrtc',
            //                         type: 'webrtc',
            //                         file: '{play_url}'
            //                     }}
            //             ]
            //         }});
            //     }}
            //     "#
            // ));
        }
    });

    rsx! {
        div { class: "drawer lg:drawer-open min-h-screen bg-base-100",
            input {
                id: "room-navigation",
                r#type: "checkbox",
                class: "drawer-toggle",
            }
            main { class: "drawer-content flex min-w-0 flex-col",
                header { class: "navbar border-b border-base-300 bg-base-100 px-4 sm:px-8",
                    div { class: "flex-none lg:hidden",
                        label {
                            r#for: "room-navigation",
                            class: "btn btn-ghost btn-square drawer-button",
                            aria_label: "打开直播间列表",
                            span { class: "icon-[mdi--menu] size-5" }
                        }
                    }
                    div { class: "min-w-0 flex-1",
                        if !resource_errors().is_empty() {
                            div {
                                class: "max-w-xl truncate text-sm text-error",
                                title: resource_errors().join("\n"),
                                for error in resource_errors().iter().take(3) {
                                    p { class: "truncate", "{error}" }
                                }
                            }
                        }
                    }
                    div { class: "flex-none",
                        if let Some(Ok(account)) = &*account.read() {
                            span { class: "mr-2 text-sm text-base-content/65", "{account.username}" }
                            button {
                                class: "btn btn-ghost btn-square",
                                title: "修改密码",
                                onclick: move |_| is_change_password_open.set(true),
                                span { class: "icon-[mdi--lock-reset] size-5" }
                            }
                            button {
                                class: "btn btn-ghost btn-square",
                                title: "退出登录",
                                onclick: move |_| logout_action.call(),
                                span { class: "icon-[mdi--logout-variant] size-5" }
                            }
                        } else {
                            button {
                                class: "btn btn-ghost btn-square",
                                title: "登录管理",
                                onclick: move |_| is_login_open.set(true),
                                span { class: "icon-[mdi--account-circle-outline] size-6" }
                            }
                        }
                    }
                }
                section { class: "flex flex-1 flex-col p-4 sm:p-6",
                    div { class: "flex min-h-0 flex-1 flex-col",
                        div { class: "flex flex-wrap items-start justify-between gap-4",
                            div {
                                div { class: "group mb-2 flex items-center gap-2",
                                    if room().is_locked {
                                        span { class: "icon-[mdi--lock] size-4 text-base-content/60" }
                                    }
                                    if room().is_live {
                                        span { class: "status status-success status-sm" }
                                    } else {
                                        span { class: "status status-neutral status-sm" }
                                        span { class: "text-xs text-base-content/55",
                                            "[离线]"
                                        }
                                    }
                                    h1 {
                                        class: "text-2xl font-bold",
                                        contenteditable: if can_manage() { "plaintext-only" } else { "false" },
                                        oninput: move |e| {
                                            if can_manage() {
                                                title_draft.set(e.value());
                                            }
                                        },
                                        onblur: move |_| async move {
                                            if can_manage() && title_draft() != room().title {
                                                update_room_action
                                                    .call(
                                                        room().stream_key.clone(),
                                                        api::rooms::UpdateRoomRequest {
                                                            title: Some(title_draft()),
                                                            lock_password: None,
                                                        },
                                                    )
                                                    .await;
                                                rooms.restart();
                                            }
                                        },
                                        "{room().title}"
                                    }
                                    if can_manage() {
                                        button {
                                            class: "btn btn-ghost btn-square btn-sm opacity-0 transition-opacity group-hover:opacity-100",
                                            title: if room().is_locked { "解锁" } else { "上锁" },
                                            onclick: move |_| {
                                                update_room_action
                                                    .call(
                                                        room().stream_key.clone(),
                                                        api::rooms::UpdateRoomRequest {
                                                            title: None,
                                                            lock_password: None,
                                                        },
                                                    );
                                                lock_override.set(Some(!room().is_locked));
                                            },
                                            span { class:
                                                if room().is_locked {
                                                    "icon-[mdi--lock] size-4"
                                                } else {
                                                    "icon-[mdi--lock-open-variant-outline] size-4"
                                                } }
                                        }
                                    }
                                }
                                p { class: "mt-1 text-sm text-base-content/60",
                                    "房主: {room().owner_username} · {room().viewer_count} 人观看"
                                }
                            }
                            div {
                                if let Some(Ok(Some(info))) = &*publish_info.read() {
                                    div { class: "dropdown dropdown-end dropdown-hover",
                                        button { class: "btn btn-ghost",
                                            span { class: "icon-[mdi--open-in-new] size-5" }
                                            "推流说明"
                                        }
                                        div { class: "card card-sm dropdown-content bg-base-100 rounded-box w-64 shadow-sm",
                                            div { class: "card-body",
                                                p {
                                                    PublishAddress {
                                                        label: "SRT",
                                                        address: info.srt_url.clone(),
                                                    }
                                                    PublishAddress {
                                                        label: "RTMP",
                                                        address: info.rtmp_url.clone(),
                                                    }
                                                    "推荐 OBS 进行推流, 设置-直播-服务 = 自定义，设置推流地址（服务器）为上述地址, 优先使用 SRT 协议, 可获得更低的延迟"
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(play_url) = play_url() {
                                    div { class: "dropdown dropdown-end dropdown-hover",
                                        button { class: "btn btn-ghost",
                                            span { class: "icon-[mdi--open-in-new] size-5" }
                                            "外部播放器"
                                        }
                                        ul { class: "dropdown-content menu border border-base-300 bg-base-100 p-2 shadow-xl",
                                            div { class: "join",
                                                input {
                                                    r#type: "radio",
                                                    class: "join-item btn btn-sm",
                                                    name: "play-mode",
                                                    checked: play_mode() == "srt",
                                                    value: "srt",
                                                    "aria-label": "SRT",
                                                    onchange: move |_| play_mode.set("srt"),
                                                }
                                                input {
                                                    r#type: "radio",
                                                    class: "join-item btn btn-sm",
                                                    name: "play-mode",
                                                    checked: play_mode() == "rtmp",
                                                    value: "rtmp",
                                                    "aria-label": "RTMP",
                                                    onchange: move |_| play_mode.set("rtmp"),
                                                }
                                                input {
                                                    r#type: "radio",
                                                    class: "join-item btn btn-sm",
                                                    name: "play-mode",
                                                    checked: play_mode() == "http-flv",
                                                    value: "http-flv",
                                                    "aria-label": "HTTP-FLV",
                                                    onchange: move |_| play_mode.set("http-flv"),
                                                }
                                            }
                                            for p in PLAYERS {
                                                li {
                                                    a {
                                                        onclick: {
                                                            let play_url = player_url(p.0, &play_url);
                                                            move |_| {
                                                                document::eval(
                                                                    &format!("window.open('{play_url}');console.log('{play_url}');"),
                                                                );
                                                            }
                                                        },
                                                        i {
                                                            class: "size-5 bg-cover",
                                                            background_image: Some(
                                                                format!(
                                                                    "url(\"https://emby-external-url.7o7o.cc/embyWebAddExternalUrl/icons/{}\")",
                                                                    p.2,
                                                                ),
                                                            ),
                                                        }
                                                        "{p.1}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "flex min-h-80 flex-1 items-center justify-center overflow-hidden bg-neutral text-neutral-content",
                            div { id: "player_id" }
                        }
                    }
                }
            }
            div { class: "drawer-side",
                label {
                    r#for: "room-navigation",
                    aria_label: "关闭直播间列表",
                    class: "drawer-overlay",
                }
                aside { class: "flex min-h-full w-72 flex-col border-r border-base-300 bg-base-200",
                    div { class: "flex items-center justify-between px-5 py-5",
                        div { class: "flex items-center gap-2",
                            span { class: "icon-[mdi--broadcast] size-6 text-primary" }
                            p { class: "font-bold", "SRS Rooms" }
                        }
                        button {
                            class: "btn btn-ghost btn-square btn-sm",
                            title: "创建直播间",
                            disabled: !is_administrator(),
                            onclick: move |_| {
                                is_create_room_open.set(true);
                            },
                            span { class: "icon-[mdi--plus] size-5" }
                        }
                    }
                    ul { class: "menu w-full gap-1 px-3",
                        li { class: "menu-title", "直播间" }
                        if let Some(Ok(rooms)) = &*rooms.read() {
                            for room in rooms {
                                RoomNavigationItem {
                                    stream_key: room.stream_key.clone(),
                                    title: room.title.clone(),
                                    is_live: room.is_live,
                                    is_locked: room.is_locked,
                                    viewer_count: room.viewer_count,
                                    selected_stream_key,
                                }
                            }
                        } else {
                            li {
                                span { class: "px-4 py-3 text-sm text-base-content/55",
                                    "正在加载直播间..."
                                }
                            }
                        }
                    }
                }
            }
        }
        if is_login_open() {
            LoginModal { is_login_open }
        }
        if is_change_password_open() {
            ChangePasswordModal { is_change_password_open }
        }
        if is_create_room_open() {
            CreateRoomModal { is_create_room_open, rooms, selected_stream_key }
        }
    }
}

#[component]
fn PublishAddress(label: &'static str, address: String) -> Element {
    rsx! {
        div { class: "mb-3",
            p { class: "mb-1 text-xs text-base-content/55", "{label}" }
            div { class: "flex items-center gap-1",
                input {
                    class: "min-w-0 flex-1 bg-base-200 px-2 py-1 text-xs",
                    value: "{address}",
                    readonly: true,
                }
                button {
                    class: "btn btn-ghost btn-square btn-xs",
                    title: "复制地址",
                    onclick: move |_| {
                        let script = format!("navigator.clipboard.writeText({address:?})");
                        spawn(async move {
                            let _ = document::eval(&script).await;
                        });
                    },
                    span { class: "icon-[mdi--content-copy] size-4" }
                }
            }
        }
    }
}

#[component]
fn RoomNavigationItem(
    stream_key: String,
    title: String,
    is_live: bool,
    is_locked: bool,
    viewer_count: i64,
    selected_stream_key: Signal<String>,
) -> Element {
    let status_class = if is_live {
        "status status-success status-sm"
    } else {
        "status status-neutral status-sm"
    };

    rsx! {
        li {
            button {
                class: "flex items-center gap-3",
                onclick: move |_| {
                    selected_stream_key.set(stream_key.clone());
                },
                span { class: "{status_class}" }
                span { class: "min-w-0 flex-1 truncate text-left", "{title}" }
                if is_locked {
                    span { class: "icon-[mdi--lock-outline] size-4 text-base-content/55" }
                } else {
                    span { class: "icon-[mdi--lock-open-variant-outline] size-4 text-base-content/55" }
                }
                span { class: "badge badge-ghost badge-sm", "{viewer_count}" }
            }
        }
    }
}

#[component]
fn LoginModal(mut is_login_open: Signal<bool>) -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut login_action = use_action(api::auth::login);

    use_effect(move || {
        if let Some(Ok(_)) = login_action.value() {
            let _ = document::eval("window.location.reload()");
        }
    });

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box max-w-sm",
                button {
                    class: "btn btn-ghost btn-square btn-sm absolute right-3 top-3",
                    title: "关闭登录",
                    onclick: move |_| is_login_open.set(false),
                    span { class: "icon-[mdi--close] size-5" }
                }
                h2 { class: "text-xl font-semibold", "登录管理" }
                form {
                    class: "mt-5 space-y-4",
                    onsubmit: move |event| {
                        event.prevent_default();
                        let login_username = username.read().trim().to_owned();
                        let login_password = password.read().to_string();
                        login_action
                            .call(
                                Form(api::auth::LoginForm {
                                    username: login_username,
                                    password: login_password,
                                }),
                            );
                    },
                    label { class: "fieldset",
                        span { class: "fieldset-legend", "用户名" }
                        input {
                            class: "input w-full",
                            r#type: "text",
                            autocomplete: "username",
                            value: "{username}",
                            oninput: move |event| username.set(event.value()),
                        }
                    }
                    label { class: "fieldset",
                        span { class: "fieldset-legend", "密码" }
                        input {
                            class: "input w-full",
                            r#type: "password",
                            autocomplete: "current-password",
                            value: "{password}",
                            oninput: move |event| password.set(event.value()),
                        }
                    }
                    if let Some(Err(error)) = login_action.value() {
                        p { class: "text-sm text-error", "{error}" }
                    }
                    button { class: "btn btn-primary w-full", r#type: "submit",
                        span { class: "icon-[mdi--login-variant] size-5" }
                        "登录"
                    }
                }
            }
            div {
                class: "modal-backdrop",
                onclick: move |_| is_login_open.set(false),
            }
        }
    }
}

#[component]
fn ChangePasswordModal(mut is_change_password_open: Signal<bool>) -> Element {
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut change_password_action = use_action(api::auth::change_password);

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box max-w-sm",
                button {
                    class: "btn btn-ghost btn-square btn-sm absolute right-3 top-3",
                    title: "关闭修改密码",
                    onclick: move |_| is_change_password_open.set(false),
                    span { class: "icon-[mdi--close] size-5" }
                }
                h2 { class: "text-xl font-semibold", "修改密码" }
                form {
                    class: "mt-5 space-y-4",
                    onsubmit: move |event| {
                        event.prevent_default();
                        change_password_action
                            .call(api::auth::ChangePasswordRequest {
                                current_password: current_password(),
                                new_password: new_password(),
                            });
                    },
                    label { class: "fieldset",
                        span { class: "fieldset-legend", "当前密码" }
                        input {
                            class: "input w-full",
                            r#type: "password",
                            autocomplete: "current-password",
                            value: "{current_password}",
                            oninput: move |event| current_password.set(event.value()),
                        }
                    }
                    label { class: "fieldset",
                        span { class: "fieldset-legend", "新密码" }
                        input {
                            class: "input w-full",
                            r#type: "password",
                            autocomplete: "new-password",
                            minlength: "10",
                            value: "{new_password}",
                            oninput: move |event| new_password.set(event.value()),
                        }
                    }
                    if let Some(Ok(_)) = change_password_action.value() {
                        p { class: "text-sm text-success", "密码已更新" }
                    }
                    if let Some(Err(error)) = change_password_action.value() {
                        p { class: "text-sm text-error", "{error}" }
                    }
                    button { class: "btn btn-primary w-full", r#type: "submit",
                        span { class: "icon-[mdi--lock-reset] size-5" }
                        "保存新密码"
                    }
                }
            }
            div {
                class: "modal-backdrop",
                onclick: move |_| is_change_password_open.set(false),
            }
        }
    }
}

#[component]
fn CreateRoomModal(
    mut is_create_room_open: Signal<bool>,
    mut rooms: Resource<anyhow::Result<Vec<api::rooms::RoomSummary>>>,
    mut selected_stream_key: Signal<String>,
) -> Element {
    let mut owner_username = use_signal(String::new);
    let mut title = use_signal(String::new);
    let mut create_account = use_action(api::rooms::create_room);

    use_effect(move || {
        if let Some(Ok(room)) = create_account.value() {
            selected_stream_key.set(room().stream_key.clone());
            rooms.restart();
        }
    });

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box max-w-lg",
                button {
                    class: "btn btn-ghost btn-square btn-sm absolute right-3 top-3",
                    title: "关闭创建直播间",
                    onclick: move |_| is_create_room_open.set(false),
                    span { class: "icon-[mdi--close] size-5" }
                }
                h2 { class: "text-xl font-semibold", "创建直播间" }
                if let Some(Ok(room)) = create_account.value() {
                    div { class: "mt-5 space-y-4",
                        p { class: "text-sm text-success",
                            "直播间和房主账号已创建。请立即保存以下信息，密码不会再次显示。"
                        }
                        ReadonlyField {
                            label: "账号名",
                            value: room().owner_username.clone(),
                        }
                        ReadonlyField {
                            label: "初始密码",
                            value: room().initial_password.clone(),
                        }
                        ReadonlyField {
                            label: "SRT 推流地址",
                            value: room().publish_info.srt_url.clone(),
                        }
                        ReadonlyField {
                            label: "RTMP 推流地址",
                            value: room().publish_info.rtmp_url.clone(),
                        }
                        div { class: "border-t border-base-300 pt-4 text-sm leading-6 text-base-content/75",
                            p { class: "font-medium text-base-content", "OBS 开播" }
                            p {
                                "打开 OBS 的“设置 > 直播”，服务选择“自定义”。将上方 SRT 或 RTMP 推流地址粘贴到“服务器”，优先使用 SRT；保存后点击“开始直播”。"
                            }
                        }
                        button {
                            class: "btn btn-primary w-full",
                            onclick: move |_| is_create_room_open.set(false),
                            "完成"
                        }
                    }
                } else {
                    form {
                        class: "mt-5 space-y-4",
                        onsubmit: move |event| {
                            event.prevent_default();
                            create_account
                                .call(api::rooms::CreateRoomRequest {
                                    owner_username: owner_username.read().trim().to_owned(),
                                    title: title.read().trim().to_owned(),
                                });
                        },
                        label { class: "fieldset",
                            span { class: "fieldset-legend", "房主账号名" }
                            input {
                                class: "input w-full",
                                r#type: "text",
                                autocomplete: "username",
                                minlength: "2",
                                maxlength: "64",
                                value: "{owner_username}",
                                oninput: move |event| owner_username.set(event.value()),
                            }
                        }
                        label { class: "fieldset",
                            span { class: "fieldset-legend", "直播间标题" }
                            input {
                                class: "input w-full",
                                r#type: "text",
                                value: "{title}",
                                oninput: move |event| title.set(event.value()),
                            }
                        }
                        if let Some(Err(error)) = create_account.value() {
                            p { class: "text-sm text-error", "{error}" }
                        }
                        button { class: "btn btn-primary w-full", r#type: "submit",
                            span { class: "icon-[mdi--plus] size-5" }
                            "创建并生成账号"
                        }
                    }
                }
            }
            div {
                class: "modal-backdrop",
                onclick: move |_| is_create_room_open.set(false),
            }
        }
    }
}

#[component]
fn ReadonlyField(label: &'static str, value: String) -> Element {
    rsx! {
        label { class: "fieldset",
            span { class: "fieldset-legend", "{label}" }
            input {
                class: "input w-full font-mono text-sm",
                value: "{value}",
                readonly: true,
            }
        }
    }
}

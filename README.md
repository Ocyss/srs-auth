# srs-auth

一个轻量的 [SRS](https://ossrs.io/) 直播鉴权项目。它提供基础账号鉴权和直播间创建能力，方便和朋友快速建立直播间、推流与观战。

## 功能

- 基础账号登录与鉴权
- 创建直播间
- 通过 SRS HTTP 回调校验推流令牌
- 通过 SRS HTTP 回调限制未授权播放

## 启动

首次启动服务前，必须同时配置初始管理员账号和密码：

```env
ADMIN_USERNAME=admin
ADMIN_PASSWORD=change-me
```

随后在项目根目录启动开发服务：

```bash
dx serve --hot-patch false
```

默认回调地址使用 `127.0.0.1:8080`；若服务运行在其他地址或端口，请同步调整下方 SRS 配置。

## SRS 配置

在 SRS 配置中加入以下内容：

```conf
http_api {
 enabled on;
 listen 1985;
 auth {
  enabled  on;
  username admin;
  password admin;
 }
}

vhost __defaultVhost__ {
 http_hooks {
  enabled      on;
  on_publish   http://127.0.0.1:8080/api/v1/streams;
  on_unpublish http://127.0.0.1:8080/api/v1/streams;
  on_play      http://127.0.0.1:8080/api/v1/sessions;
  on_stop      http://127.0.0.1:8080/api/v1/sessions;

  # 可选：用于房间上锁时踢出当前观看者。
  # 未启用时仅会禁止后续播放请求。
  # on_dvr http://127.0.0.1:8080/api/v1/play;
  # on_hls http://127.0.0.1:8080/api/v1/play;
 }
}
```

强烈建议启用 `http_api.auth`，避免 SRS HTTP API 被扫描或未授权访问。请将示例中的 `username` 和 `password` 替换为自己的高强度凭据。

`on_dvr` 和 `on_hls` 是可选回调。启用后，房间加锁时可用于踢出正在观看的用户；未启用时，鉴权只会阻止新的播放请求。当前完整的房间上锁流程尚未完成，因此这两个回调暂时不是必需项。

## TODO

- 完善房间上锁流程，包括阻止新观众进入并踢出当前观看者。

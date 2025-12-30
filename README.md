# @lightsing/llc-zh-cn

> **Disclaimer / 免责声明**：
>
> This is an **unofficial** NPM distribution of the [LocalizeLimbusCompany][llc-repo] assets. [This repo][project-branch] is automatically synchronized with the original repository's releases.
>
> 本项目是 [LocalizeLimbusCompany][llc-repo] 资产的**非官方** NPM 分发版本。[本仓库][project-branch]通过自动化脚本与原仓库的 Release 保持同步。
>
> This project is not affiliated with, endorsed by, or associated with either **Project Moon** or the **Localize Limbus Company Team**.
>
> 本项目与 **Project Moon** 或 **边狱公司本地化项目组/都市零协会汉化组** 均无任何关联、背书或合作关系。

[![npm version](https://img.shields.io/npm/v/@lightsing/llc-zh-cn.svg)](https://www.npmjs.com/package/@lightsing/llc-zh-cn)
[![License](https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-black.svg)](https://creativecommons.org/licenses/by-nc-sa/4.0/)
[![Sync Status](https://github.com/lightsing/llc-rs/actions/workflows/sync.yml/badge.svg)](https://github.com/lightsing/llc-rs/actions)

---

## 📌 Origin / 原作信息

- **Original Repository / 原始仓库**: [LocalizeLimbusCompany/LocalizeLimbusCompany][llc-repo]
- **Original Team / 原始团队**: Localize Limbus Company Team (边狱公司本地化项目组/都市零协会汉化组)
- **License / 许可协议**: [CC BY-NC-SA 4.0][cc-license] / [署名—非商业性使用—相同方式共享 4.0 国际版][cc-license-zh]

## 🔢 Versioning / 版本规范

The version number of this NPM package is automatically synchronized with the original repository's Release publication timestamps to ensure traceability.

本项目 NPM 包的版本号与原仓库的 Release 发布时间同步，以确保版本的可追溯性。

The tag of the original repository's Release is in extra field of `package.json` as `githubTag`.

原仓库 Release 的标签会被记录在 `package.json` 的额外字段 `githubTag` 中。

## 🛠 How it Works / 自动化原理

This project uses GitHub Actions for full automation without manual intervention:

本项目通过 GitHub Actions 实现全自动同步，无需人工干预：

1. **Monitoring / 监控**：Check the original repository's latest Release every hour. / 每小时检查一次原仓库的最新 Release。
2. **Verification / 校验**：Compare the NPM remote version number with the GitHub Release publish time to determine if there is a new version. / 对比 NPM 远程版本号与 GitHub Release 发布时间，判断是否有新版本。
3. **Distribution / 分发**：If a new version is detected, the script will automatically download the `.zip` asset, repackage it, and publish it to NPM. / 若检测到新版本，脚本会自动下载 `.zip` 资产，重新打包并发布至 NPM。

## ⚖️ License / 授权协议

The assets distributed by this project are licensed under the [**Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International (CC BY-NC-SA 4.0)** license][cc-license].

本项目分发的资产内容遵循 [**Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International (CC BY-NC-SA 4.0)** 协议][cc-license-zh]。

You are free to:

您可以自由地：

- **Share / 共享** — copy and redistribute the material in any medium or format / 在任何媒介以任何形式复制、发行本作品
- **Adapt / 演绎** — remix, transform, and build upon the material / 修改、转换或以本作品为基础进行创作

The licensor cannot revoke these freedoms as long as you follow the license terms.

只要你遵守许可协议条款，许可人就无法收回你的这些权利。

As long as you follow the following terms:

惟须遵守下列条件：

- **Attribution / 署名** — You must give appropriate credit, provide a link to the license, and indicate if changes were made. You may do so in any reasonable manner, but not in any way that suggests the licensor endorses you or your use. / 您必须给出适当的署名，提供指向本许可协议的链接，同时标明是否（对原始作品）作了修改。您可以用任何合理的方式来署名，但是不得以任何方式暗示许可人为您或您的使用背书。
- **NonCommercial / 非商业性使用** — You may not use the material for commercial purposes. / 您不得将本作品用于商业目的。
- **ShareAlike / 相同方式共享** — If you remix, transform, or build upon the material, you must distribute your contributions under the same license as the original.
  / 如果您再混合、转换或者基于本作品进行创作，您必须基于与原先许可协议相同的许可协议 分发您贡献的作品。

**No additional restrictions / 无附加限制** — You may not apply legal terms or technological measures that legally restrict others from doing anything the license permits. / 您不得适用法律术语或者 技术措施 从而限制其他人做许可协议允许的事情。

For more details, please refer to the `LICENSE` file in the project or visit the [Attribution-NonCommercial-ShareAlike 4.0 International Legal Code](https://creativecommons.org/licenses/by-nc-sa/4.0/legalcode.en).

详情请参阅项目中的 `LICENSE` 文件或访问[署名—非商业性使用—相同方式共享 4.0 协议国际版法律文本](https://creativecommons.org/licenses/by-nc-sa/4.0/legalcode.zh-hans)。

---

## 📝 Maintenance / 维护说明

If there are any issues with the NPM packaging process, please open an issue in this repository.
如果 NPM 打包流程出现问题（例如解压路径错误），请在[此仓库][project-issues]提交 Issue。

If there are any issues regarding the translation content itself, please report them directly to the [Localize Limbus Company Team][llc-issues].
关于翻译内容本身的问题，请直接反馈给[边狱公司本地化项目组/都市零协会汉化组][llc-issues]。

[llc-repo]: https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany
[project-branch]: https://github.com/lightsing/llc-rs/tree/llc-sync
[cc-license]: https://creativecommons.org/licenses/by-nc-sa/4.0/
[cc-license-zh]: https://creativecommons.org/licenses/by-nc-sa/4.0/deed.zh-hans
[project-issues]: https://github.com/lightsing/llc-rs/issues
[llc-issues]: https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany/issues

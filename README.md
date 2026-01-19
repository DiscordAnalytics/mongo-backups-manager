# MongoDB Backups Manager

> [!IMPORTANT]
> This repository is under active development

MongoDB Backups Manager (shortly MBM) is a simple tool that allows you to create backups of your MongoDB database.

# Installation
> [!IMPORTANT]
> Real installation instructions will be provided on v1.0.0 release.

## Ubuntu/Debian
```shell
sudo apt install mbm -y
```
## Fedora
```shell
sudo dnf install mbm -y
```

# Development setup
First, you'll need to install the Rust Toolchain:
```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Then, clone the repo and build the project:
```shell
git clone https://github.com/DiscordAnalytics/mongo-backups-manager
cd mongo-backups-manager
cargo build --bin mbm
```
Read [CODE_OF_CONDUCT.md](./.github/CODE_OF_CONDUCT.md) and [./.github/CONTRIBUTING.md](./.github/CONTRIBUTING.md), create a branch and start coding 😎
```shell
git branch feat/super-cool-feature
git checkout feat/super-cool-feature
```

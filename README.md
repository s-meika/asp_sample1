# asp_sample1
Sample program for TOPPERS/ASP

[topper_asp]（https://github.com/s-meika/toppers_asp）クレートを使ったサンプルプログラムです。
TOPPERS/ASPに同梱されているsample1プログラムをRustで実装しています。

# 動作確認済み環境

- ハードウェア
    - 本体：[Wio Terminal](https://wiki.seeedstudio.com/jp/Wio-Terminal-Getting-Started/)
    - デバッガ:[Seeeduio XIAO](https://wiki.seeedstudio.com/jp/Seeeduino-XIAO/)
- ビルド環境
    - MacBook Pro, 2017
    - macOS Catalina 10.15.7
    - rustc 1.55.0-nightly (607d6b00d 2021-06-15)
    - cargo 1.54.0-nightly (44456677b 2021-06-12)
    - cargo make
    - gcc version 9.2.1 20191025 (release) [ARM/arm-9-branch revision 277599] (GNU Tools for Arm Embedded Processors 9-2019-q4-major)
    - hf2

# ビルド方法
## ソースのクローン

```
git clone https://github.com/s-meika/toppers_asp.git
git clone https://github.com/s-meika/asp_sample1.git
```

## カーネルのビルド
```
cd asp_sample1
cargo make prebuild
cargo make build
```

## 形式の変換

現状生成されるelfファイルはそのまま書き込んでも動作しません。

(デバッガの場合はelf形式で動作します)

```
arm-none-eabi-objcopy -O binary  target/thumbv7em-none-eabihf/debug/asp_sample1 asp_sample1.bin
```

# 書き込み

```
hf2 flash --address 0x4000 --file asp_sample1.bin
```


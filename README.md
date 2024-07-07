# os-beginner

[ゼロからのOS自作入門](https://book.mynavi.jp/ec/products/detail/id=121220)をRustで実装していく予定。
ただし、これを書いている時点では中身を全く読んでいないので、挫折する可能性もあり。
そうなった場合は素直にC++で書く。

## 起動方法

qemu、[cargo-make](https://github.com/sagiegurari/cargo-make) をインストールし

```bash
cargo make
```

を実行すると起動する。

このとき [apps](./mikan-os/apps) に含まれるアプリは `$APPS_DIR` で指定されたディレクトリにコピーされる。
また `$RESOURCE_DIR` を指定すれば、ルートディレクトリに `$RESOURCE_DIR` 以下のファイルが配置される。

非 ASCII 文字の表示にデフォルトではルートに配置された `ipag.ttf` が使われる。

## ライセンス

デフォルトで使用される [IPA フォント](https://moji.or.jp/ipafont/ipa00303/) のライセンスは
./IPA_Font_License_Agreement_v1.0.txt
に配置されている。

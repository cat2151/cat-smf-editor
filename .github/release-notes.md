$name v$version (Windows x64)

## 配布物
- `$name-$version-windows-x64.zip`: exe と $package_files。VC++ ランタイムを静的リンクしているので、Microsoft Visual C++ 再頒布可能パッケージは不要

## この版について
- GitHub Actions が毎日自動でビルドし、公開している（下の commit がどれも変わらない日は作り直さない）
- 通っているのは自動の検査（ビルド、VC++ ランタイムのリンク検査、テスト）だけで、人による動作確認はしていない
- uapmd は上流の main を使うので、作者が手元で確かめた版と違う場合がある（下の「ビルドに使った commit」）

<details>
<summary>ビルドに使った commit</summary>

$commits

</details>

$build_inputs

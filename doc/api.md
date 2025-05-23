# addParametricGraph API

## 概要
RustからJavaScriptに提供されるAPIで、任意のパラメトリック曲線グラフを描画できます。

## シグネチャ
```js
addParametricGraph(f: function(t) -> [x, y], range: { min: number, max: number, delta?: number }, name: string)
```

- `f`: tを引数に[x, y]座標を返す関数
- `range`: tの範囲を指定するオブジェクト。
    - `delta`が指定された場合は`min`から`max`まで`delta`刻みで点を生成。
- `name`: グラフのラベル（凡例やツールチップに表示）

## 使い方サンプル
```js
addParametricGraph(
    function(t) { return [a * Math.cos(t), b * Math.sin(t)]; },
    { min: 0, max: 2 * Math.PI, delta: 0.1 },
    "媒介変数曲線"
);
```

## Rust側のパラメータの受け渡し
- Rustから`a`, `b`などの変数をJSグローバルに渡しているので、JS内で直接参照可能です。

## 備考
- サンプルでは1本のみ返していますが、今後複数グラフの追加や、他のAPI拡張も可能です。
- `delta`を使うと点の間隔を明示的に制御できます。

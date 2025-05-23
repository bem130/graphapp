# JavaScript API Documentation

## addSlider API

### 概要
スライダーを追加するためのAPI。パラメータの値を動的に調整できます。

### シグネチャ
```js
addSlider(name: string, params: {
    min?: number,     // 最小値（省略時: 0.0）
    max?: number,     // 最大値（省略時: 1.0）
    step?: number,    // 増減の単位（省略時: 0.1）
    default?: number  // 初期値（省略時: 0.0）
})
```

### 使用例
```js
addSlider("radius", {
    min: 0.1,
    max: 2.0,
    step: 0.1,
    default: 1.0
});
```

## addParametricGraph API

### 概要
パラメトリック曲線をグラフに描画するためのAPI。

### シグネチャ
```js
addParametricGraph(
    name: string,           // グラフの名前
    f: (t: number) => [number, number],  // パラメトリック関数
    range: {
        min?: number,       // パラメータの最小値（省略時: 0.0）
        max?: number,       // パラメータの最大値（省略時: 2π）
        delta?: number,     // 点の間隔（省略可）
        num_points?: number // 点の数（deltaが指定されていない場合。省略時: 500）
    }
)
```

### 使用例
```js
// 楕円を描画
addParametricGraph(
    "楕円",
    function(t) { return [a * Math.cos(t), b * Math.sin(t)]; },
    { min: 0, max: 2 * Math.PI, num_points: 1000 }
);
```

## addVector API

### 概要
ベクトル（矢印）を描画するためのAPI。始点とベクトルを関数として指定できます。

### シグネチャ
```js
addVector(
    name: string,                         // ベクトルの名前
    start_f: (t: number) => [number, number],  // 始点を計算する関数
    vector_f: (t: number) => [number, number], // ベクトルを計算する関数
    t: number                                  // パラメータ値
)
```

### 使用例
```js
// 円周上の点における接線ベクトルを描画
addVector(
    "接線ベクトル",
    function(t) { return [Math.cos(t), Math.sin(t)]; },    // 円周上の点
    function(t) { return [-Math.sin(t), Math.cos(t)]; },   // 接線ベクトル
    Math.PI / 4  // π/4の位置に描画
);
```

## グローバル変数

スライダーで設定された値は、JavaScript内でグローバル変数として利用できます。
変数名はスライダー作成時の`name`パラメータで指定した名前になります。

### 例
```js
addSlider("a", { min: 0, max: 1 });  // スライダーを作成
console.log(a);  // スライダーの現在値を参照
```

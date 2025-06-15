# JavaScript API リファレンス

このドキュメントは、グラフ描画アプリケーション Neknaj Graph Plotter の JavaScript 環境で使用できる API について説明します。

## グローバル関数

### `setup()`

スクリプトの初期化時に一度だけ呼び出される関数です。
この関数内で、スライダー、チェックボックス、カラーピッカーなどの UI 要素を定義します。

### `draw()`

グラフの描画処理を行う関数です。
`setup()` の初回実行後、および UI 要素（スライダー、チェックボックス、カラーピッカー）の値が変更されるたびに呼び出されます。
この関数内で `addParametricGraph` や `addVector` を呼び出してグラフ要素を描画します。

## UI 要素定義 API

これらの関数は `setup()` 内で呼び出して、ユーザーが操作できる UI 要素を定義します。
定義された UI 要素の値は、対応する `name` でグローバル変数として `draw()` 関数内からアクセスできます。

### `addSlider(name: String, params: Object)`

スライダーを定義します。

*   `name` (String): スライダーの名前。この名前でグローバル変数が作成され、スライダーの現在の値（数値）を保持します。
*   `params` (Object): スライダーのパラメータを指定するオブジェクト。
    *   `min` (Number, optional): 最小値。デフォルトは `0.0`。
    *   `max` (Number, optional): 最大値。デフォルトは `1.0`。
    *   `step` (Number, optional): 値の刻み幅。デフォルトは `0.001`。
    *   `default` (Number, optional): 初期値。デフォルトは `0.0`。

**例:**

```js
addSlider('amplitude', { min: 0, max: 10, step: 0.5, default: 5 });
// これにより、グローバル変数 `amplitude` が利用可能になる
```

### `addCheckbox(name: String, label: String, params?: Object)`

チェックボックスを定義します。

*   `name` (String): チェックボックスの名前。この名前でグローバル変数が作成され、チェックボックスの状態（boolean: `true` または `false`）を保持します。
*   `label` (String): チェックボックスの横に表示されるラベル文字列。
*   `params` (Object, optional): チェックボックスのパラメータを指定するオブジェクト。
    *   `default` (Boolean, optional): 初期状態。デフォルトは `true`。

**例:**

```js
addCheckbox('isVisible', '表示する', { default: false });
// これにより、グローバル変数 `isVisible` が利用可能になる
```

### `addColorpicker(name: String, params?: Object)`

カラーピッカーを定義します。

*   `name` (String): カラーピッカーの名前。この名前でグローバル変数が作成され、選択された色を `[r, g, b]` 形式の配列（各要素は 0-255 の数値）で保持します。
*   `params` (Object, optional): カラーピッカーのパラメータを指定するオブジェクト。
    *   `default` (Array<Number>, optional): 初期色を `[r, g, b]` 形式で指定。デフォルトは `[255, 255, 255]` (白)。

**例:**

```js
addColorpicker('graphColor', { default: [0, 0, 255] }); // 青色
// これにより、グローバル変数 `graphColor` が [r, g, b] 配列として利用可能になる
```

## 描画 API

これらの関数は主に `draw()` 内で呼び出して、グラフ上に図形を描画します。

### `addParametricGraph(name: String, func: Function, range: Object, style?: Object)`

媒介変数表示された曲線を描画します。

*   `name` (String): 曲線の名前（凡例などで使用）。
*   `func` (Function): 媒介変数 `t` を引数に取り、座標 `[x, y]` を返す関数。
*   `range` (Object): 媒介変数の範囲と精度を指定するオブジェクト。
    *   `min` (Number, optional): `t` の最小値。デフォルトは `0.0`。
    *   `max` (Number, optional): `t` の最大値。デフォルトは `2 * Math.PI`。
    *   `num_points` (Number, optional): 描画点数。`delta` が指定されていない場合に使用。デフォルトは `500`。
    *   `delta` (Number, optional): `t` の刻み幅。指定された場合 `num_points` は無視される。
*   `style` (Object, optional): 線のスタイルを指定するオブジェクト。
    *   `color` (Array<Number>, optional): 線の色を `[r, g, b]` (各 0-255) で指定。デフォルトは `[200, 100, 0]`。
    *   `weight` (Number, optional): 線の太さ。デフォルトは `1.5`。

**例:**

```js
addParametricGraph(
    'リサージュ図形',
    function(t) { return [Math.sin(3 * t), Math.cos(2 * t)]; },
    { min: 0, max: 2 * Math.PI, num_points: 1000 },
    { color: [255, 165, 0], weight: 2.0 }
);
```

### `addVector(name: String, start_func: Function, vec_func: Function, t: Number, style?: Object)`

ベクトル（矢印）を描画します。

*   `name` (String): ベクトルの名前。
*   `start_func` (Function): 媒介変数 `t` を引数に取り、ベクトルの始点 `[x, y]` を返す関数。
*   `vec_func` (Function): 媒介変数 `t` を引数に取り、ベクトルの成分 `[dx, dy]` を返す関数。終点は `[x + dx, y + dy]` となります。
*   `t` (Number): `start_func` と `vec_func` を評価する際の媒介変数の値。
*   `style` (Object, optional): 矢印のスタイルを指定するオブジェクト。
    *   `color` (Array<Number>, optional): 矢印の色を `[r, g, b]` (各 0-255) で指定。デフォルトは `[0, 150, 200]`。
    *   `weight` (Number, optional): 矢印の線の太さ。デフォルトは `1.5`。

**例:**

```js
let t_val = 1.5; // スライダーなどで動的に変更可能
addVector(
    '接線ベクトル',
    function(t) { return [Math.cos(t), Math.sin(t)]; }, // 円周上の点
    function(t) { return [-Math.sin(t), Math.cos(t)]; }, // 円の接線方向
    t_val,
    { color: [0, 255, 0], weight: 2.0 }
);
```

### `addPolygon(name: String, points: Array<Array<Number>>, style?: Object)`

多角形（ポリゴン）を塗りつぶして描画します。

*   `name` (String): 多角形の名前（凡例などで使用）。
*   `points` (Array<Array<Number>>): 頂点座標の配列。各要素は `[x, y]` 形式の配列。
*   `style` (Object, optional): スタイル指定オブジェクト。
    *   `color` (Array<Number>, optional): 線の色 `[r, g, b]` (各 0-255)。デフォルトは `[0, 0, 0]`。
    *   `weight` (Number, optional): 線の太さ。デフォルトは `1.5`。

**例:**

```js
addPolygon(
    '三角形',
    [ [0, 0], [1, 0], [0.5, 1] ],
    { color: [255, 0, 0], weight: 2.0 }
);
```

## コンソール出力

JavaScript 内から Rust のコンソールに情報を出力できます。

### `console.log(...args)`

引数を JSON 文字列化して標準出力に出力します。

**例:**

```js
console.log('デバッグ情報:', myVariable, { value: 42 });
```

### `console.error(...args)`

引数を JSON 文字列化して標準エラー出力に赤色で出力します。

**例:**

```js
console.error('エラーが発生しました:', errorObject);
```
**内部API (直接呼び出し非推奨)**

*   `stdout(String)`: 文字列を標準出力へ。`console.log` の内部で使用。
*   `stderr(String)`: 文字列を標準エラーへ。`console.error` の内部で使用。


# サンプルコード

```js
function setup() {
    addSlider('radius', { min: 0.5, max: 5.0, step: 0.001, default: 1.0 });
    addColorpicker('lineColor', { default: [255, 0, 0] });
    addCheckbox('showCircle', '円を表示する', { default: true });
}

function draw() {
    if (showCircle) {
        addParametricGraph(
            '円',
            function(t) { return [radius * Math.cos(t), radius * Math.sin(t)]; },
            { min: 0, max: 2 * Math.PI, num_points: 100 },
            { color: lineColor, weight: 2.0 }
        );
    }
}
```

```js
function setup() {
    addSlider('amplitude1', { min: 0.1, max: 3.0, step: 0.01, default: 1.0 });
    addSlider('frequency1', { min: 1, max: 10, step: 0.01, default: 3 });
    addSlider('amplitude2', { min: 0.1, max: 3.0, step: 0.01, default: 0.5 });
    addSlider('frequency2', { min: 1, max: 10, step: 0.01, default: 5 });
    addColorpicker('graph1Color', { default: [255, 0, 0] });
    addColorpicker('graph2Color', { default: [0, 0, 255] });
    addCheckbox('showDebug', 'デバッグ情報を表示', { default: false });
}

function draw() {
    // グラフ1: 振幅と周波数が制御可能なサインカーブ
    addParametricGraph(
        'サインカーブ1',
        function(t) { return [t, amplitude1 * Math.sin(frequency1 * t)]; },
        { min: -5, max: 5, num_points: 2000 },
        { color: graph1Color, weight: 1.5 }
    );

    // グラフ2: 振幅と周波数が制御可能なコサインカーブ
    addParametricGraph(
        'コサインカーブ2',
        function(t) { return [t, amplitude2 * Math.cos(frequency2 * t)]; },
        { min: -5, max: 5, num_points: 2000 },
        { color: graph2Color, weight: 1.5 }
    );

    if (showDebug) {
        console.log("グラフ1 - 振幅:", amplitude1, "周波数:", frequency1);
        console.log("グラフ2 - 振幅:", amplitude2, "周波数:", frequency2);
    }
}
```

```js
function setup() {
    addSlider('tValue', { min: -5.0, max: 5.0, step: 0.01, default: 0.0 });
    addColorpicker('parabolaColor', { default: [0, 100, 200] });
    addColorpicker('vectorColor', { default: [200, 50, 0] });
}

function draw() {
    // 放物線 y = x^2 を媒介変数表示で描画 (x = t, y = t^2)
    addParametricGraph(
        '放物線',
        function(t) { return [t, t * t]; },
        { min: -5, max: 5, num_points: 200 },
        { color: parabolaColor, weight: 1.5 }
    );

    // スライダーで指定された t の値における点と接線ベクトル
    // 点: (t, t^2)
    // 接線ベクトル: (dx/dt, dy/dt) = (1, 2t)
    addVector(
        '接線ベクトル',
        function(t) { return [t, t * t]; },
        function(t) { return [1, 2 * t]; },
        tValue,
        { color: vectorColor, weight: 2.5 }
    );

    console.log("現在の t の値:", tValue);
}
```
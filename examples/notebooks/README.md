# liel example notebooks

Notebook は「動くデモ」と「長めの利用例」を確認する場所です。基本的な API は `examples/` と `docs/` を先に見てください。

| Notebook | 目的 | 先に見る人 |
|---|---|---|
| `social_network_analysis.ipynb` | 小さなソーシャルグラフで、CRUD・QueryBuilder・pandas・可視化・永続化を一通り確認する | 最初に notebook で触りたい人 |
| `01_wikipedia_graph_tour.ipynb` | 公開データセットを取得し、Wikipedia リンクグラフを bulk insert して traversal / shortest path / 可視化を試す | 少し大きな実データで見せたい人 |

`.executed.ipynb` は出力確認用の実行済みスナップショットです。編集や再実行は、出力なしの `.ipynb` を使ってください。

## データ

- `examples/notebooks/data/social_network/` は小さな同梱データです。
- `01_wikipedia_graph_tour.ipynb` は SNAP Wikispeedia の公開データを使います。初回実行時に `examples/notebooks/data/wikispeedia/` へ展開します。
- `.liel` 生成物はデモ用です。再生成できるものは、必要に応じて削除して構いません。

## 位置づけ

- API の最短確認: `examples/01_quickstart.py`
- 公開データの bulk insert: `examples/03_bulk_import.py`
- Agent memory の最小例: `examples/07_agent_memory.py`
- 長めの体験: この `examples/notebooks/` フォルダ

(function () {
  const MAX_GRAPH_NODES = 300;
  const MAX_GRAPH_EDGES = 600;
  const SAMPLE_EXPORT_URL = "./fixtures/trace-why-postgres.export.json";

  const state = {
    exportData: null,
    nodeTable: null,
    edgeTable: null,
    graph: null,
    statusMessage: null,
  };

  const DEFAULT_EXPORT = {
    edge_count: 9,
    export_version: 1,
    liel_format: "1.0",
    node_count: 7,
    nodes: [
      {
        id: 1,
        labels: ["Task"],
        properties: {
          key: "task:billing-service",
          status: "done",
          title: "Implement billing service",
          trace_prompt: "Why PostgreSQL for billing?",
        },
      },
      {
        id: 2,
        labels: ["Option"],
        properties: {
          key: "option:postgres",
          key_factor: "ACID transactions",
          summary: "ACID transactions and strong consistency",
          title: "Use PostgreSQL",
        },
      },
      {
        id: 3,
        labels: ["Option"],
        properties: {
          key: "option:dynamodb",
          rejection_note: "better for scale, not for this use case",
          summary: "Scalable and operationally simple",
          title: "Use DynamoDB",
        },
      },
      {
        id: 4,
        labels: ["Bug"],
        properties: {
          key: "bug:duplicate-charge",
          severity: "high",
          summary: "Previous eventual consistency issue caused double billing",
          title: "Duplicate charge incident",
        },
      },
      {
        id: 5,
        labels: ["Requirement"],
        properties: {
          key: "requirement:audit-trail",
          source: "SOC2 review",
          summary: "Billing records must be traceable and immutable",
          title: "Audit trail requirement",
        },
      },
      {
        id: 6,
        labels: ["Decision"],
        properties: {
          key: "decision:billing-postgres",
          reason:
            "ACID transactions and strong consistency; Prevents duplicate charges after the prior incident; Supports auditability and immutable billing records",
          title: "Choose PostgreSQL for billing",
        },
      },
      {
        id: 7,
        labels: ["File"],
        properties: {
          key: "file:billing/db.py",
          path: "billing/db.py",
          summary: "Billing persistence layer",
        },
      },
    ],
    edges: [
      {
        id: 1,
        from_node: 2,
        to_node: 6,
        label: "SUPPORTS",
        properties: { why: "ACID and consistency needs" },
      },
      {
        id: 2,
        from_node: 3,
        to_node: 6,
        label: "REJECTED_FOR",
        properties: { why: "fit for scale, weaker transactional fit" },
      },
      {
        id: 3,
        from_node: 4,
        to_node: 6,
        label: "MOTIVATED",
        properties: { incident: "duplicate-charge", severity: "high" },
      },
      {
        id: 4,
        from_node: 5,
        to_node: 6,
        label: "REQUIRED",
        properties: { requirement: "audit-trail" },
      },
      {
        id: 5,
        from_node: 6,
        to_node: 7,
        label: "IMPLEMENTED_IN",
        properties: { file_role: "persistence-layer" },
      },
      {
        id: 6,
        from_node: 1,
        to_node: 4,
        label: "LEARNED_FROM",
        properties: { evidence: "incident review" },
      },
      {
        id: 7,
        from_node: 1,
        to_node: 5,
        label: "CONSTRAINED_BY",
        properties: { source: "SOC2" },
      },
      {
        id: 8,
        from_node: 1,
        to_node: 3,
        label: "CONSIDERED",
        properties: { outcome: "rejected" },
      },
      {
        id: 9,
        from_node: 1,
        to_node: 2,
        label: "CONSIDERED",
        properties: { outcome: "selected" },
      },
    ],
  };

  const statusEl = document.getElementById("status");
  const summaryEl = document.getElementById("summaryCards");
  const traceEl = document.getElementById("tracePreview");

  document.getElementById("exportFile").addEventListener("change", async (e) => {
    state.exportData = await readJsonFile(e.target.files[0]);
    renderAll();
  });


  document.getElementById("resetToSample").addEventListener("click", async () => {
    await loadBundledSample();
    renderAll();
  });

  function readJsonFile(file) {
    if (!file) return Promise.resolve(null);
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        try {
          resolve(JSON.parse(reader.result));
        } catch (err) {
          reject(err);
        }
      };
      reader.onerror = reject;
      reader.readAsText(file);
    }).catch((err) => {
      setStatus(`Failed to parse ${file.name}: ${String(err)}`, true);
      return null;
    });
  }

  function setStatus(message, isError = false) {
    statusEl.textContent = message;
    statusEl.style.color = isError ? "#b91c1c" : "#1f2937";
  }

  function asLabelText(labels) {
    if (!Array.isArray(labels) || labels.length === 0) return "";
    return labels.join(", ");
  }

  function formatProps(props) {
    if (!props || typeof props !== "object") return "—";
    if (Object.keys(props).length === 0) return "—";
    try {
      return JSON.stringify(props);
    } catch {
      return "—";
    }
  }

  function displayPrimaryText(nodeLike) {
    const p = nodeLike?.properties || {};
    return (
      p.name ||
      p.title ||
      p.path ||
      p.key ||
      (nodeLike?.id != null ? `node:${nodeLike.id}` : "")
    );
  }

  function updateSummary() {
    const cards = [];
    const e = state.exportData || {};

    cards.push({ key: "Format", value: e.liel_format || "-" });
    cards.push({ key: "Export version", value: e.export_version ?? "-" });
    cards.push({ key: "Nodes", value: e.node_count ?? "-" });
    cards.push({ key: "Edges", value: e.edge_count ?? "-" });
    cards.push({ key: "Node labels", value: countDistinctNodeLabels(e.nodes) });
    cards.push({ key: "Edge labels", value: countDistinctEdgeLabels(e.edges) });

    summaryEl.innerHTML = cards
      .map(
        (card) =>
          `<div class="card"><div class="k">${escapeHtml(card.key)}</div><div class="v">${escapeHtml(
            String(card.value)
          )}</div></div>`
      )
      .join("");
  }

  function countDistinctNodeLabels(nodes) {
    if (!Array.isArray(nodes)) return "-";
    const labels = new Set();
    for (const node of nodes) {
      if (Array.isArray(node.labels)) {
        for (const label of node.labels) labels.add(label);
      }
    }
    return labels.size;
  }

  function countDistinctEdgeLabels(edges) {
    if (!Array.isArray(edges)) return "-";
    const labels = new Set();
    for (const edge of edges) {
      if (edge && typeof edge.label === "string" && edge.label.length > 0) {
        labels.add(edge.label);
      }
    }
    return labels.size;
  }

  function updateTables() {
    const nodes = Array.isArray(state.exportData?.nodes) ? state.exportData.nodes : [];
    const edges = Array.isArray(state.exportData?.edges) ? state.exportData.edges : [];

    const nodeRows = nodes.map((n) => ({
      id: n.id,
      labels: asLabelText(n.labels),
      key: n.properties?.key || "",
      name: displayPrimaryText(n),
      properties: formatProps(n.properties),
    }));

    const edgeRows = edges.map((ed) => ({
      id: ed.id,
      from_node: ed.from_node,
      to_node: ed.to_node,
      label: ed.label || "",
      properties: formatProps(ed.properties),
    }));

    if (!state.nodeTable) {
      state.nodeTable = new Tabulator("#nodeTable", {
        height: 280,
        data: nodeRows,
        layout: "fitColumns",
        placeholder: "Load export JSON to view nodes",
        columns: [
          { title: "id", field: "id", width: 80, sorter: "number" },
          { title: "labels", field: "labels" },
          { title: "key", field: "key" },
          { title: "name/title", field: "name" },
          { title: "properties", field: "properties" },
        ],
      });
    } else {
      state.nodeTable.replaceData(nodeRows);
    }

    if (!state.edgeTable) {
      state.edgeTable = new Tabulator("#edgeTable", {
        height: 280,
        data: edgeRows,
        layout: "fitColumns",
        placeholder: "Load export JSON to view edges",
        columns: [
          { title: "id", field: "id", width: 80, sorter: "number" },
          { title: "from", field: "from_node", width: 80, sorter: "number" },
          { title: "to", field: "to_node", width: 80, sorter: "number" },
          { title: "label", field: "label" },
          { title: "properties", field: "properties" },
        ],
      });
    } else {
      state.edgeTable.replaceData(edgeRows);
    }
  }

  function updateGraph() {
    const container = document.getElementById("graphCanvas");
    const nodes = Array.isArray(state.exportData?.nodes) ? state.exportData.nodes : [];
    const edges = Array.isArray(state.exportData?.edges) ? state.exportData.edges : [];

    const nodeSlice = nodes.slice(0, MAX_GRAPH_NODES).map((n) => ({
      id: n.id,
      label: displayPrimaryText(n),
      title: `${asLabelText(n.labels)}\n${formatProps(n.properties)}`,
      shape: "dot",
    }));

    const allowedNodeIds = new Set(nodeSlice.map((n) => n.id));
    const edgeSlice = edges
      .filter((e) => allowedNodeIds.has(e.from_node) && allowedNodeIds.has(e.to_node))
      .slice(0, MAX_GRAPH_EDGES)
      .map((e) => ({
        id: e.id,
        from: e.from_node,
        to: e.to_node,
        label: e.label || "",
        arrows: "to",
      }));

    const data = {
      nodes: new vis.DataSet(nodeSlice),
      edges: new vis.DataSet(edgeSlice),
    };
    const options = {
      physics: false,
      interaction: { hover: true },
      edges: { font: { align: "middle", size: 10 } },
      nodes: { font: { size: 12 } },
      layout: { improvedLayout: true },
    };

    if (!state.graph) {
      state.graph = new vis.Network(container, data, options);
    } else {
      state.graph.setData(data);
    }
  }

  function updateTrace() {
    const nodes = Array.isArray(state.exportData?.nodes)
      ? state.exportData.nodes.length
      : 0;
    const edges = Array.isArray(state.exportData?.edges)
      ? state.exportData.edges.length
      : 0;
    traceEl.textContent =
      "Use CLI for path narratives:\n\n" +
      "  liel trace <file.liel> --from <NODE_ID> --to <NODE_ID> --format json\n\n" +
      `Loaded export contains ${nodes} nodes and ${edges} edges.`;
  }

  function renderAll() {
    updateSummary();
    updateTables();
    updateGraph();
    updateTrace();

    const loaded = [state.exportData ? "export" : null].filter(Boolean);
    setStatus(
      state.statusMessage ||
        (loaded.length === 0
          ? "No files loaded yet."
          : `Loaded: ${loaded.join(", ")} JSON`)
    );
    state.statusMessage = null;
  }

  async function fetchBundledSample() {
    const response = await fetch(SAMPLE_EXPORT_URL, { cache: "no-store" });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} while loading ${SAMPLE_EXPORT_URL}`);
    }
    return response.json();
  }

  async function loadBundledSample() {
    // Prefer the checked-in export fixture so docs, tests, and viewer code share
    // the same contract sample. Keep the embedded object as a file:// fallback
    // for browsers that block local fetches when the HTML is opened directly.
    try {
      state.exportData = await fetchBundledSample();
    } catch (err) {
      state.exportData = JSON.parse(JSON.stringify(DEFAULT_EXPORT));
      state.statusMessage =
        `Loaded embedded fallback sample because ${SAMPLE_EXPORT_URL} could not be fetched: ${String(err)}`;
    }
  }

  function escapeHtml(text) {
    return text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  loadBundledSample().then(renderAll);
})();

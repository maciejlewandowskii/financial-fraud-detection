const PAGE_SIZE = 15;
const ID = "TransactionID";
const MAIN_FIELDS = [
  { col: "TransactionID", label: "ID transakcji" },
  { col: "TransactionDT", label: "Czas (sekundy)", number: true },
  { col: "TransactionAmt", label: "Kwota (USD)", number: true, required: true },
  { col: "ProductCD", label: "Produkt", options: ["W", "C", "R", "H", "S"] },
  { col: "card1", label: "Karta (card1)", number: true },
  { col: "card4", label: "Sieć karty", options: ["visa", "mastercard", "american express", "discover"] },
  { col: "card6", label: "Typ karty", options: ["debit", "credit", "charge card", "debit or credit"] },
  { col: "addr1", label: "Region (addr1)", number: true },
  { col: "dist1", label: "Odległość (dist1)", number: true },
  { col: "P_emaildomain", label: "E-mail płacącego" },
  { col: "R_emaildomain", label: "E-mail odbiorcy" },
  { col: "C1", label: "Licznik C1", number: true },
  { col: "D1", label: "Dni od D1", number: true },
];
const TABLE_COLUMNS = [
  { col: "TransactionID", label: "ID" },
  { col: "TransactionDT", label: "Czas", format: formatDt },
  { col: "TransactionAmt", label: "Kwota", format: (v) => fmtMoney.format(Number(v)), num: true },
  { col: "ProductCD", label: "Produkt" },
  { col: "card4", label: "Karta" },
  { col: "card6", label: "Typ" },
  { col: "P_emaildomain", label: "E-mail" },
];
const $ = (id) => document.getElementById(id);
const fmtInt = new Intl.NumberFormat("pl-PL");
const fmtMoney = new Intl.NumberFormat("pl-PL", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

const form = $("tx-form");
const state = { offset: 0, total: 0, onlyFraud: false, rows: [], selected: null, label: null, columns: null };

function formatDt(value) {
  const seconds = Number(value);
  const day = Math.floor(seconds / 86400);
  const time = new Date((seconds % 86400) * 1000).toISOString().slice(11, 16);
  return `dzień ${day}, ${time}`;
}

function showError(err) {
  $("error").textContent = err ? String(err) : "";
  $("error").hidden = !err;
}

function makeField({ col, label, number, required, options }) {
  const wrap = document.createElement("div");
  wrap.className = "field";
  let input;
  if (options) {
    input = document.createElement("select");
    input.add(new Option("—", ""));
    for (const o of options) input.add(new Option(o));
  } else {
    input = document.createElement("input");
    input.placeholder = " ";
    if (number) Object.assign(input, { type: "number", step: "any" });
  }
  Object.assign(input, { name: col, required: !!required });
  const text = document.createElement("label");
  text.textContent = label;
  wrap.append(input, text);
  return wrap;
}

function buildForm(columns) {
  const present = columns ? MAIN_FIELDS.filter((f) => columns.includes(f.col)) : MAIN_FIELDS;
  $("main-fields").replaceChildren(...present.map(makeField));
  const mainCols = new Set(present.map((f) => f.col));
  const rest = (columns ?? []).filter((c) => !mainCols.has(c));
  $("extra-fields").replaceChildren(...rest.map((col) => makeField({ col, label: col })));
  $("extra-count").textContent = rest.length;
  $("extra").hidden = rest.length === 0;
}

function tableColumns() {
  const present = TABLE_COLUMNS.filter((c) => state.columns.includes(c.col));
  return present.length ? present : state.columns.slice(0, 6).map((col) => ({ col, label: col }));
}

function buildTableHead() {
  const th = (text, num) => Object.assign(document.createElement("th"), { textContent: text, className: num ? "num" : "" });
  $("table-head").replaceChildren(th("#"), ...tableColumns().map((c) => th(c.label, c.num)), th("Etykieta"));
}

function readForm() {
  return Object.fromEntries([...new FormData(form)].map(([k, v]) => [k, String(v).trim()]));
}

function fillForm(tx) {
  for (const input of form.elements) {
    if (!input.name) continue;
    const value = tx[input.name] ?? "";
    if (input.tagName === "SELECT" && ![...input.options].some((o) => o.value === value)) {
      input.add(new Option(value));
    }
    input.value = value;
  }
}

function labelBadge(isFraud) {
  const badge = document.createElement("span");
  badge.className = "badge " + (isFraud ? "fraud" : "legit");
  badge.textContent = isFraud ? "oszustwo" : "prawidłowa";
  return badge;
}

function renderRows() {
  const tbody = $("rows");
  tbody.replaceChildren();
  const columns = tableColumns();
  for (const row of state.rows) {
    const tx = row.transaction;
    const tr = document.createElement("tr");
    tr.classList.toggle("selected", row.index === state.selected);
    const index = document.createElement("td");
    index.textContent = fmtInt.format(row.index + 1);
    index.className = "muted";
    tr.append(index);
    for (const c of columns) {
      const td = document.createElement("td");
      const value = tx[c.col];
      td.textContent = value ? (c.format ? c.format(value) : value) : "—";
      if (c.num) td.className = "num";
      tr.append(td);
    }
    const labelCell = document.createElement("td");
    labelCell.append(labelBadge(row.is_fraud));
    tr.append(labelCell);
    tr.addEventListener("click", () => selectRow(row));
    tbody.append(tr);
  }

  const pages = Math.max(1, Math.ceil(state.total / PAGE_SIZE));
  const page = Math.floor(state.offset / PAGE_SIZE) + 1;
  const end = Math.min(state.offset + PAGE_SIZE, state.total);
  $("range").textContent = state.total
    ? `${fmtInt.format(state.offset + 1)}–${fmtInt.format(end)} z ${fmtInt.format(state.total)}`
    : "Brak rekordów";
  $("page-number").value = page;
  $("page-number").max = pages;
  $("page-count").textContent = fmtInt.format(pages);
  $("prev-page").disabled = page <= 1;
  $("next-page").disabled = page >= pages;
}

async function loadPage(offset) {
  showError("");
  try {
    const page = await api.listTransactions(Math.max(0, offset), PAGE_SIZE, state.onlyFraud);
    Object.assign(state, { offset: page.offset, total: page.total, rows: page.rows });
    renderRows();
  } catch (err) {
    showError(err);
  }
}

function renderPrediction(prediction) {
  const percent = (prediction.fraud_probability * 100).toFixed(1);
  $("verdict").textContent = prediction.is_fraud ? "Podejrzana — oszustwo" : "Wygląda na prawidłową";
  $("verdict").className = "verdict " + (prediction.is_fraud ? "fraud" : "legit");
  $("meter-fill").style.width = percent + "%";
  $("meter-fill").classList.toggle("fraud", prediction.is_fraud);
  $("probability").textContent = `${percent}%`;
  $("result-id").textContent = form.elements[ID]?.value ? `ID ${form.elements[ID].value}` : "";

  $("truth").hidden = !state.label;
  if (state.label) {
    $("truth-label").replaceChildren(labelBadge(state.label.isFraud));
    const match = state.label.isFraud === prediction.is_fraud;
    $("truth-match").textContent = match ? "✓ Model trafił" : "✗ Model się pomylił";
    $("truth-match").className = "match " + (match ? "ok" : "miss");
  }
  $("result-empty").hidden = true;
  $("result-body").hidden = false;
}

async function check() {
  showError("");
  try {
    renderPrediction(await api.predict(readForm()));
  } catch (err) {
    showError(err);
  }
}

function selectRow(row) {
  state.selected = row.index;
  state.label = { isFraud: row.is_fraud };
  fillForm(row.transaction);
  renderRows();
  check();
}

async function loadDataset() {
  showError("");
  $("dataset-status").textContent = "Indeksowanie…";
  try {
    const info = await api.openDataset();
    const name = info.path.split(/[\\/]/).pop();
    $("dataset-status").textContent =
      `${name} · ${fmtInt.format(info.rows)} rekordów · ${fmtInt.format(info.fraud_rows)} oszustw`;
    state.columns = info.columns;
    buildForm(info.columns);
    buildTableHead();
    $("browser-empty").hidden = true;
    $("browser-body").hidden = false;
    $("random-tx").disabled = false;
    $("only-fraud").disabled = false;
    state.selected = null;
    await loadPage(0);
  } catch (err) {
    $("dataset-status").textContent = "Brak danych";
    $("browser-empty").textContent = "Nie udało się wczytać danych treningowych.";
    showError(err);
  }
}

$("random-tx").addEventListener("click", async () => {
  showError("");
  try {
    const picked = await api.randomTransaction(state.onlyFraud);
    state.selected = picked.row.index;
    await loadPage(Math.floor(picked.position / PAGE_SIZE) * PAGE_SIZE);
    selectRow(picked.row);
  } catch (err) {
    showError(err);
  }
});

$("only-fraud").addEventListener("change", (event) => {
  state.onlyFraud = event.target.checked;
  loadPage(0);
});

$("prev-page").addEventListener("click", () => loadPage(state.offset - PAGE_SIZE));
$("next-page").addEventListener("click", () => loadPage(state.offset + PAGE_SIZE));
$("page-number").addEventListener("change", (event) => {
  const page = Math.max(1, Math.floor(Number(event.target.value)) || 1);
  loadPage((page - 1) * PAGE_SIZE);
});

form.addEventListener("input", () => { state.label = null; });
form.addEventListener("submit", (event) => {
  event.preventDefault();
  check();
});

buildForm(null);

api.modelInfo().then((info) => {
  $("model-name").textContent = `Model: ${info.name}`;
});

loadDataset();

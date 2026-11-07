const { invoke } = window.__TAURI__.core;

window.api = {
  modelInfo: () => invoke("model_info"),
  predict: (transaction) => invoke("predict", { transaction }),
  openDataset: (path) => invoke("open_dataset", { path }),
  listTransactions: (offset, limit, onlyFraud) => invoke("list_transactions", { offset, limit, onlyFraud }),
  randomTransaction: (onlyFraud) => invoke("random_transaction", { onlyFraud }),
  pickCsv: () => window.__TAURI__.dialog.open({
    multiple: false,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  }),
};

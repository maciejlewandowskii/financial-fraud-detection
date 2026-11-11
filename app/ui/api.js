const { invoke } = window.__TAURI__.core;

window.api = {
  modelInfo: () => invoke("model_info"),
  predict: (transaction) => invoke("predict", { transaction }),
  openDataset: () => invoke("open_dataset"),
  listTransactions: (offset, limit, onlyFraud) => invoke("list_transactions", { offset, limit, onlyFraud }),
  randomTransaction: (onlyFraud) => invoke("random_transaction", { onlyFraud }),
};

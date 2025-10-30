/**
 * Utility functions for the Sharp sidecar
 * @module utils
 */

const logger = require("./logger");
const files = require("./files");

module.exports = {
  ...logger,
  ...files,
};

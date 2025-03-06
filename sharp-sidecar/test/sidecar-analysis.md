# Sidecar Communication Analysis Report

*Generated on: 2025-03-06T16:33:49.210Z*

## Summary Statistics

- **Total messages captured**: 43
- **Message types**:
  - `progress`: 10 messages (23.3%)
  - `detailed_progress`: 10 messages (23.3%)
  - `progress_update`: 11 messages (25.6%)

## Progress Messages Analysis

- **Progress message types**:
  - `start`: 10 messages (100.0%)

### Progress Message Samples

#### `start` Sample:

```json
{
  "type": "progress",
  "progressType": "start",
  "taskId": "D:\\image-optimizer\\sharp-sidecar\\test\\images\\test-image_05.jpg",
  "workerId": 4,
  "metrics": {
    "completedTasks": 0,
    "totalTasks": 10,
    "queueLength": 10
  }
}
```


## Other Message Types

### `detailed_progress` Messages (10)

Sample:

```json
{
  "type": "detailed_progress",
  "fileName": "test-image_03.jpg",
  "taskId": "D:\\image-optimizer\\sharp-sidecar\\test\\images\\test-image_03.jpg",
  "optimizationMetrics": {
    "originalSize": 112004,
    "optimizedSize": 19479,
    "savedBytes": 92525,
    "compressionRatio": "82.61",
    "format": "jpeg"
  },
  "batchMetrics": {
    "completedTasks": 1,
    "totalTasks": 10,
    "progressPercentage": 10
  },
  "formattedMessage": "test-image_03.jpg optimized (90.36 KB saved / 82.61% compression) - Progress: 10% (1/10)"
}
```

### `progress_update` Messages (11)

Sample:

```json
{
  "type": "progress_update",
  "completedTasks": 1,
  "totalTasks": 10,
  "progressPercentage": 10,
  "status": "processing",
  "metadata": {}
}
```


## Communication Flow

```
Sharp Sidecar                      Rust Backend
-------------                      ------------
      |                                 |
      | unknown ------->                |
      |                                 |
      | unknown ------->                |
      |                                 |
      | unknown ------->                |
      |                                 |
      | unknown ------->                |
      |                                 |
      | unknown ------->                |
      |                                 |
      | ...more messages...             |
      |                                 |
      | unknown ------->                |
```


## Data Size Analysis

- **Total data transferred**: 8.01 KB
- **Average message size**: 190.86046511627907 bytes

### Largest Messages

1. **Type**: `detailed_progress` - **Size**: 459 bytes
2. **Type**: `detailed_progress` - **Size**: 456 bytes
3. **Type**: `detailed_progress` - **Size**: 456 bytes

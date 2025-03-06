# Sidecar Communication Analysis Report

*Generated on: 2025-03-06T15:16:13.511Z*

## Summary Statistics

- **Total messages captured**: 43
- **Message types**:
  - `progress`: 20 messages (46.5%)
  - `progress_update`: 11 messages (25.6%)

## Progress Messages Analysis

- **Progress message types**:
  - `start`: 10 messages (50.0%)
  - `complete`: 10 messages (50.0%)

### Progress Message Samples

#### `start` Sample:

```json
{
  "type": "progress",
  "progressType": "start",
  "taskId": "D:\\image-optimizer\\sharp-sidecar\\test\\images\\test-image_07.jpg",
  "workerId": 6,
  "metrics": {
    "completedTasks": 0,
    "totalTasks": 10,
    "queueLength": 10
  }
}
```

#### `complete` Sample:

```json
{
  "type": "progress",
  "progressType": "complete",
  "taskId": "D:\\image-optimizer\\sharp-sidecar\\test\\images\\test-image_03.jpg",
  "workerId": 2,
  "result": {
    "path": "C:\\Users\\jesse\\AppData\\Local\\Temp\\sharp-sidecar-test-output\\test-image_03.jpg",
    "original_size": 112004,
    "optimized_size": 19479,
    "saved_bytes": 92525,
    "compression_ratio": "82.61",
    "format": "jpeg",
    "success": true,
    "error": null,
    "workerId": 2
  },
  "metrics": {
    "completedTasks": 0,
    "totalTasks": 10,
    "queueLength": 10
  }
}
```


## Other Message Types

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

- **Total data transferred**: 8.02 KB
- **Average message size**: 191.09302325581396 bytes

### Largest Messages

1. **Type**: `progress/complete` - **Size**: 457 bytes
2. **Type**: `progress/complete` - **Size**: 457 bytes
3. **Type**: `progress/complete` - **Size**: 457 bytes

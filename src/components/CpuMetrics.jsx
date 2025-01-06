import { useState, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";

function CpuMetrics() {
  const [metrics, setMetrics] = useState([]);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const workerMetrics = await invoke('get_worker_metrics');
        setMetrics(workerMetrics);
      } catch (error) {
        console.error('Error fetching metrics:', error);
      }
    };

    // Update metrics every second
    const interval = setInterval(fetchMetrics, 1000);
    return () => clearInterval(interval);
  }, []);

  if (metrics.length === 0) return null;

  return (
    <div className="cpu-metrics">
      <h3>Worker Metrics</h3>
      <div className="cpu-metrics__grid">
        {metrics.map((metric, index) => (
          <div key={index} className="cpu-metrics__worker">
            <div className="cpu-metrics__title">Worker {metric.thread_id + 1}</div>
            <div className="cpu-metrics__stats">
              <div className="cpu-metrics__stat">
                <span>CPU:</span>
                <span>{metric.cpu_usage.toFixed(1)}%</span>
              </div>
              <div className="cpu-metrics__stat">
                <span>Tasks:</span>
                <span>{metric.task_count}</span>
              </div>
              <div className="cpu-metrics__stat">
                <span>Avg Time:</span>
                <span>{metric.avg_processing_time.toFixed(2)}s</span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default CpuMetrics; 
import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { mkdir } from "@tauri-apps/plugin-fs";
import { dirname, join } from "@tauri-apps/api/path";
import { invoke } from "@tauri-apps/api/core";

function Dropzone() {
  const [isDragging, setIsDragging] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [optimizationStats, setOptimizationStats] = useState(null);

  useEffect(() => {
    const unlisten = listen("tauri://drag-enter", () => {
      setIsDragging(true);
    });

    const unlistenCancel = listen("tauri://drag-leave", () => {
      setIsDragging(false);
    });

    const unlistenDrop = listen("tauri://drag-drop", async (event) => {
      setIsDragging(false);
      const paths = event.payload.paths;

      if (paths && paths.length > 0) {
        try {
          setIsProcessing(true);
          const startTime = performance.now();
          let processedFiles = 0;

          // Create all required directories first
          await Promise.all(paths.map(async (path) => {
            const parentDir = await dirname(path);
            const optimizedPath = await join(parentDir, 'optimized');
            await mkdir(optimizedPath, { recursive: true });
          }));

          // Process images in parallel with Promise.all
          await Promise.all(paths.map(async (path) => {
            try {
              await invoke('optimize_image', { path });
              processedFiles++;
              
              setOptimizationStats(prev => ({
                totalFiles: paths.length,
                processedFiles,
                elapsedTime: ((performance.now() - startTime) / 1000).toFixed(2)
              }));
            } catch (error) {
              console.error('Error processing image:', error);
            }
          }));

        } catch (error) {
          console.error('Error processing images:', error);
        } finally {
          setIsProcessing(false);
        }
      }
    });

    return () => {
      unlisten.then((f) => f());
      unlistenCancel.then((f) => f());
      unlistenDrop.then((f) => f());
    };
  }, []);

  return (
    <div>
      <div
        style={{
          border: `2px dashed ${isDragging ? "#4a9eff" : "#ccc"}`,
          borderRadius: "4px",
          padding: "20px",
          textAlign: "center",
          background: isDragging ? "rgba(74, 158, 255, 0.1)" : "transparent",
          transition: "all 0.2s ease",
        }}
      >
        <p>{isProcessing ? 'Processing...' : 'Drag and drop files here'}</p>
      </div>
      
      {optimizationStats && (
        <div style={{ marginTop: "20px", textAlign: "center" }}>
          <p>Processed: {optimizationStats.processedFiles} of {optimizationStats.totalFiles} files</p>
          <p>Total time: {optimizationStats.elapsedTime} seconds</p>
        </div>
      )}
    </div>
  );
}

export default Dropzone;

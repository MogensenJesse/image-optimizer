The optimization workflow follows these steps:
1. **Image Selection**: User selects images via drag-and-drop or file picker
2. **Task Creation**: Frontend creates optimization tasks with settings and sends them to the backend
3. **Task Processing**: Tasks are validated, batched, and distributed to Sharp workers
4. **Result Handling**: Optimization results return to the frontend with statistics and file paths

```mermaid
flowchart TD
    %% User Interaction
    User([User]) -->|Selects Images| UI1[Drag & Drop Area]
    User -->|Configures Settings| UI2[Settings Panel]
    
    %% Frontend Processing
    UI1 -->|File Paths| FE1[File Validation]
    UI2 -->|Quality/Format Settings| FE2[Task Creation]
    FE1 -->|Valid Files| FE2
    FE2 -->|Task Objects| FE3[Tauri Invoke API]
    
    %% Backend Processing
    FE3 -->|optimize_images Command| BE1[Command Handler]
    BE1 -->|Task Queue| BE2[Process Pool]
    BE2 -->|Distribute Tasks| BE3[Worker Management]
    
    %% Sidecar Processing
    BE3 -->|JSON Task Messages| SC1[Sharp Sidecar]
    SC1 -->|Worker Assignment| SC2[Worker Threads]
    SC2 -->|Image Processing| SC3[Sharp Pipeline]
    SC3 -->|Format-specific Processing| SC4[Optimization]
    
    %% Results Flow
    SC4 -->|Optimization Results| SC5[Results Collection]
    SC5 -->|Progress Events| BE4[Event Handling]
    BE4 -->|Tauri Events| FE4[Progress Tracking]
    SC5 -->|Final Results| BE5[Results Aggregation]
    BE5 -->|Command Response| FE5[Results Display]
    
    %% UI Updates
    FE4 -->|Progress Updates| UI3[Progress Bar]
    FE5 -->|Statistics| UI4[Results Summary]
    
    %% File System Operations
    SC4 -.->|Write Optimized Files| FS[File System]
    UI4 -.->|Open Result Folder| FS
    
    %% Styling
    classDef frontend fill:#d4f1f9,stroke:#05a0c8,stroke-width:2px
    classDef backend fill:#ffe6cc,stroke:#f7931e,stroke-width:2px
    classDef sidecar fill:#e6f5d0,stroke:#8bc34a,stroke-width:2px
    classDef ui fill:#f9d4e7,stroke:#d81b60,stroke-width:2px
    classDef storage fill:#e6e6e6,stroke:#757575,stroke-width:2px
    classDef user fill:#f5f5f5,stroke:#424242,stroke-width:1px
    
    %% Apply styles
    class FE1,FE2,FE3,FE4,FE5 frontend
    class BE1,BE2,BE3,BE4,BE5 backend
    class SC1,SC2,SC3,SC4,SC5 sidecar
    class UI1,UI2,UI3,UI4 ui
    class FS storage
    class User user
```

### 1.2 Frontend (React) 